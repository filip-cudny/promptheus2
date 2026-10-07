import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';
import Clutter from 'gi://Clutter';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const PLACE_WINDOW_TIMEOUT_MS = 1000;
const OBJECT_PATH = '/com/promptheus/Shell';
const TOAST_DURATIONS_MS = {success: 2000, error: 4000, info: 2000, warning: 3000};
const TOAST_MARGIN = 20;
const TOAST_GAP = 14;
const TOAST_OPACITY = 204;
const TOAST_SHOW_ANIMATION_MS = 150;
const TOAST_HIDE_ANIMATION_MS = 150;
const WIDGET_WIDTH = 240;
const WIDGET_HEIGHT = 44;
const WIDGET_BOTTOM_MARGIN = 24;
const WIDGET_STATES = ['recording', 'paused', 'processing', 'done'];
const WIDGET_BAR_FACTORS = [0.55, 0.8, 0.65, 1, 0.75, 0.9, 0.6, 0.85, 0.5];
const WIDGET_BAR_MIN_HEIGHT = 4;
const WIDGET_BAR_MAX_HEIGHT = 24;
const WIDGET_BAR_SMOOTHING = 0.5;
const WIDGET_SPIN_MS = 1000;
const WIDGET_BUTTON_HOVER = 'rgba(255,255,255,0.14)';

function pointInRect(x, y, rect) {
    return x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height;
}

function resolveWidgetPosition(hasPosition, x, y, workAreas, pointerArea) {
    if (hasPosition && workAreas.some(area => pointInRect(x, y, area)))
        return {x, y, stored: true};
    return {
        x: Math.round(pointerArea.x + (pointerArea.width - WIDGET_WIDTH) / 2),
        y: pointerArea.y + pointerArea.height - WIDGET_BOTTOM_MARGIN - WIDGET_HEIGHT,
        stored: false,
    };
}

function formatElapsed(elapsedMs) {
    const seconds = Math.floor(elapsedMs / 1000);
    return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
}

const INTERFACE_XML = `
<node>
  <interface name="com.promptheus.Shell">
    <method name="SetShortcuts">
      <arg type="a{sas}" direction="in" name="bindings"/>
      <arg type="as" direction="out" name="failed"/>
    </method>
    <method name="GetPointer">
      <arg type="i" direction="out" name="x"/>
      <arg type="i" direction="out" name="y"/>
      <arg type="i" direction="out" name="work_x"/>
      <arg type="i" direction="out" name="work_y"/>
      <arg type="i" direction="out" name="work_width"/>
      <arg type="i" direction="out" name="work_height"/>
    </method>
    <method name="PlaceWindow">
      <arg type="s" direction="in" name="title"/>
      <arg type="i" direction="in" name="x"/>
      <arg type="i" direction="in" name="y"/>
      <arg type="b" direction="in" name="activate"/>
      <arg type="b" direction="out" name="placed"/>
    </method>
    <method name="PlaceWindowAnchored">
      <arg type="s" direction="in" name="title"/>
      <arg type="i" direction="in" name="right"/>
      <arg type="i" direction="in" name="bottom"/>
      <arg type="b" direction="in" name="activate"/>
      <arg type="b" direction="out" name="placed"/>
    </method>
    <method name="GetFocusedWmClass">
      <arg type="s" direction="out" name="wm_class"/>
    </method>
    <method name="ShowToast">
      <arg type="s" direction="in" name="level"/>
      <arg type="s" direction="in" name="title"/>
      <arg type="s" direction="in" name="message"/>
      <arg type="b" direction="in" name="monochromatic"/>
    </method>
    <method name="ShowRecordingWidget">
      <arg type="i" direction="in" name="x"/>
      <arg type="i" direction="in" name="y"/>
      <arg type="b" direction="in" name="has_position"/>
    </method>
    <method name="UpdateRecordingWidget">
      <arg type="s" direction="in" name="state"/>
      <arg type="d" direction="in" name="level"/>
      <arg type="u" direction="in" name="elapsed_ms"/>
    </method>
    <method name="HideRecordingWidget"/>
    <signal name="ShortcutActivated">
      <arg type="s" name="action"/>
    </signal>
    <signal name="RecordingWidgetAction">
      <arg type="s" name="action"/>
    </signal>
    <signal name="RecordingWidgetMoved">
      <arg type="i" name="x"/>
      <arg type="i" name="y"/>
    </signal>
    <signal name="Ready"/>
  </interface>
</node>`;

export default class PromptheusExtension extends Extension {
    enable() {
        this._grabbed = new Map();
        this._pending = new Map();
        this._anchors = new Map();
        this._watchId = 0;
        this._toasts = [];
        this._toastMonitor = 0;
        this._restackId = 0;
        this._widget = null;
        this._dbus = Gio.DBusExportedObject.wrapJSObject(INTERFACE_XML, this);
        this._dbus.export(Gio.DBus.session, OBJECT_PATH);
        this._acceleratorId = global.display.connect(
            'accelerator-activated',
            (display, action, device, timestamp) => this._onAcceleratorActivated(action));
        this._dbus.emit_signal('Ready', null);
    }

    disable() {
        if (this._acceleratorId) {
            global.display.disconnect(this._acceleratorId);
            this._acceleratorId = 0;
        }
        this._unwatchSender();
        this._ungrabAll();
        for (const pending of [...this._pending.values()])
            pending.finish(false);
        for (const anchor of [...this._anchors.values()])
            this._releaseAnchor(anchor);
        this._clearToasts();
        this._removeRecordingWidget();
        this._pending = null;
        this._anchors = null;
        this._grabbed = null;
        this._dbus.unexport();
        this._dbus = null;
    }

    SetShortcutsAsync(params, invocation) {
        try {
            const [bindings] = params;
            this._ungrabAll();
            const failed = [];
            for (const [action, accelerators] of Object.entries(bindings)) {
                for (const accelerator of accelerators) {
                    const id = global.display.grab_accelerator(accelerator, Meta.KeyBindingFlags.NONE);
                    if (id === Meta.KeyBindingAction.NONE) {
                        failed.push(accelerator);
                        continue;
                    }
                    Main.wm.allowKeybinding(
                        Meta.external_binding_name_for_action(id),
                        Shell.ActionMode.NORMAL | Shell.ActionMode.OVERVIEW);
                    this._grabbed.set(id, action);
                }
            }
            this._watchSender(invocation.get_sender());
            invocation.return_value(new GLib.Variant('(as)', [failed]));
        } catch (e) {
            logError(e, 'Promptheus: SetShortcuts failed');
            invocation.return_dbus_error('com.promptheus.Shell.Error', String(e));
        }
    }

    GetPointer() {
        const [x, y] = global.get_pointer();
        const monitor = global.display.get_current_monitor();
        const area = Main.layoutManager.getWorkAreaForMonitor(monitor);
        return [x, y, area.x, area.y, area.width, area.height];
    }

    PlaceWindowAsync(params, invocation) {
        const [title, x, y, activate] = params;
        this._placeOrWait('PlaceWindow', title, invocation,
            win => this._placeNow(win, x, y, activate));
    }

    PlaceWindowAnchoredAsync(params, invocation) {
        const [title, right, bottom, activate] = params;
        this._placeOrWait('PlaceWindowAnchored', title, invocation,
            win => this._anchorWindow(win, title, right, bottom, activate));
    }

    GetFocusedWmClass() {
        return global.display.focus_window?.get_wm_class() ?? '';
    }

    ShowToast(level, title, message, monochromatic) {
        const duration = TOAST_DURATIONS_MS[level];
        if (duration === undefined)
            throw new Error(`Unknown toast level: ${level}`);
        const actor = this._buildToast(level, title, message, monochromatic);
        if (this._toasts.length === 0)
            this._toastMonitor = global.display.get_current_monitor();
        const toast = {actor, timerId: 0};
        this._toasts.push(toast);
        toast.timerId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, duration, () => {
            toast.timerId = 0;
            this._hideToast(toast);
            return GLib.SOURCE_REMOVE;
        });
        if (!this._restackId) {
            this._restackId = global.compositor.get_laters().add(Meta.LaterType.BEFORE_REDRAW, () => {
                this._restackId = 0;
                this._restackToasts();
                return GLib.SOURCE_REMOVE;
            });
        }
        Main.layoutManager.uiGroup.add_child(actor);
        actor.ease({
            opacity: TOAST_OPACITY,
            scale_x: 1,
            scale_y: 1,
            duration: TOAST_SHOW_ANIMATION_MS,
            mode: Clutter.AnimationMode.EASE_OUT_EXPO,
        });
    }

    _hideToast(toast) {
        toast.actor.set_pivot_point(0.5, 0.5);
        toast.actor.ease({
            opacity: 0,
            scale_x: 0.8,
            scale_y: 0.8,
            duration: TOAST_HIDE_ANIMATION_MS,
            mode: Clutter.AnimationMode.EASE_OUT_QUAD,
            onStopped: () => this._removeToast(toast),
        });
    }

    _removeToast(toast) {
        if (!this._toasts.includes(toast))
            return;
        this._toasts = this._toasts.filter(t => t !== toast);
        toast.actor.destroy();
        this._restackToasts();
    }

    _buildToast(level, title, message, monochromatic) {
        const actor = new St.BoxLayout({
            style: 'width: 300px; background-color: #ffffff; border: 1px solid rgba(200,200,200,0.9); ' +
                'border-radius: 8px; padding: 12px 16px; spacing: 10px; ' +
                'font-family: "Noto Sans", sans-serif; box-shadow: 0 4px 12px rgba(0,0,0,0.15);',
            opacity: 0,
            scale_x: 0.01,
            scale_y: 0.05,
            reactive: false,
        });
        actor.set_pivot_point(0.5, 1.0);
        const file = this.dir.get_child('icons').get_child(`${level}-${monochromatic ? 'mono' : 'color'}.svg`);
        actor.add_child(new St.Icon({
            gicon: new Gio.FileIcon({file}),
            icon_size: 20,
            y_align: Clutter.ActorAlign.CENTER,
        }));
        const text = new St.BoxLayout({vertical: true, x_expand: true});
        text.add_child(new St.Label({
            text: title,
            style: 'color: #1a1a1a; font-weight: 600; font-size: 14px;',
        }));
        if (message !== '') {
            text.add_child(new St.Label({
                text: message,
                style: 'color: rgba(0,0,0,0.65); font-size: 13px;',
            }));
        }
        actor.add_child(text);
        return actor;
    }

    _restackToasts() {
        const area = Main.layoutManager.getWorkAreaForMonitor(this._toastMonitor);
        let bottom = area.y + area.height - TOAST_MARGIN;
        for (const {actor} of [...this._toasts].reverse()) {
            const [, width] = actor.get_preferred_width(-1);
            const [, height] = actor.get_preferred_height(width);
            const y = bottom - height;
            actor.set_position(area.x + area.width - TOAST_MARGIN - width, y);
            bottom = y - TOAST_GAP;
        }
    }

    _clearToasts() {
        const toasts = this._toasts;
        this._toasts = [];
        for (const toast of toasts) {
            if (toast.timerId)
                GLib.Source.remove(toast.timerId);
            toast.actor.remove_all_transitions();
            toast.actor.destroy();
        }
        if (this._restackId) {
            global.compositor.get_laters().remove(this._restackId);
            this._restackId = 0;
        }
    }

    ShowRecordingWidget(x, y, hasPosition) {
        if (!this._widget)
            this._widget = this._buildRecordingWidget();
        const widget = this._widget;
        widget.heights.fill(WIDGET_BAR_MIN_HEIGHT);
        widget.elapsed = formatElapsed(0);
        this._setWidgetState(widget, 'recording');
        const monitors = Main.layoutManager.monitors;
        const workAreas = monitors.map((_monitor, i) => Main.layoutManager.getWorkAreaForMonitor(i));
        const pointerArea = Main.layoutManager.getWorkAreaForMonitor(global.display.get_current_monitor());
        const position = resolveWidgetPosition(hasPosition, x, y, workAreas, pointerArea);
        widget.actor.set_position(position.x, position.y);
        console.log(`Promptheus: recording widget shown at ${position.x},${position.y} ` +
            `stored=${position.stored}`);
    }

    UpdateRecordingWidget(state, level, elapsedMs) {
        if (!WIDGET_STATES.includes(state))
            throw new Error(`Unknown recording widget state: ${state}`);
        const widget = this._widget;
        if (!widget)
            throw new Error('Recording widget is not shown');
        widget.elapsed = formatElapsed(elapsedMs);
        if (state !== widget.state)
            this._setWidgetState(widget, state);
        else if (state === 'recording')
            this._updateWidgetBars(widget, level);
        if (widget.timeLabel && widget.timeLabel.text !== widget.elapsed)
            widget.timeLabel.text = widget.elapsed;
    }

    HideRecordingWidget() {
        this._removeRecordingWidget();
    }

    _removeRecordingWidget() {
        const widget = this._widget;
        if (!widget)
            return;
        this._widget = null;
        this._endWidgetDrag(widget);
        this._clearWidgetContent(widget);
        widget.actor.remove_all_transitions();
        Main.layoutManager.removeChrome(widget.actor);
        widget.actor.destroy();
        console.log('Promptheus: recording widget hidden');
    }

    _emitRecordingWidgetAction(action) {
        this._dbus.emit_signal('RecordingWidgetAction', new GLib.Variant('(s)', [action]));
    }

    _buildRecordingWidget() {
        const actor = new St.BoxLayout({
            style: `width: ${WIDGET_WIDTH}px; height: ${WIDGET_HEIGHT}px; border-radius: ${WIDGET_HEIGHT / 2}px; ` +
                'background-color: rgba(28,28,30,0.92); padding: 0 14px; color: #e5e5e7; ' +
                'font-family: "Noto Sans", sans-serif; font-size: 13px;',
            reactive: true,
            can_focus: false,
            track_hover: false,
        });
        const widget = {
            actor,
            state: null,
            elapsed: formatElapsed(0),
            heights: WIDGET_BAR_FACTORS.map(() => WIDGET_BAR_MIN_HEIGHT),
            bars: [],
            timeLabel: null,
            spinner: null,
            dragId: 0,
        };
        actor.connect('button-press-event', (_actor, event) => this._beginWidgetDrag(widget, event));
        Main.layoutManager.addTopChrome(actor);
        return widget;
    }

    _beginWidgetDrag(widget, event) {
        if (event.get_button() !== Clutter.BUTTON_PRIMARY)
            return Clutter.EVENT_PROPAGATE;
        this._endWidgetDrag(widget);
        const [pointerX, pointerY] = event.get_coords();
        const offsetX = pointerX - widget.actor.x;
        const offsetY = pointerY - widget.actor.y;
        widget.dragId = global.stage.connect('captured-event', (_stage, e) => {
            const type = e.type();
            if (type === Clutter.EventType.MOTION) {
                const [x, y] = e.get_coords();
                widget.actor.set_position(Math.round(x - offsetX), Math.round(y - offsetY));
                return Clutter.EVENT_STOP;
            }
            if (type === Clutter.EventType.BUTTON_RELEASE) {
                this._endWidgetDrag(widget);
                const x = Math.round(widget.actor.x);
                const y = Math.round(widget.actor.y);
                console.log(`Promptheus: recording widget dragged to ${x},${y}`);
                this._dbus.emit_signal('RecordingWidgetMoved', new GLib.Variant('(ii)', [x, y]));
                return Clutter.EVENT_STOP;
            }
            return Clutter.EVENT_PROPAGATE;
        });
        return Clutter.EVENT_STOP;
    }

    _endWidgetDrag(widget) {
        if (widget.dragId) {
            global.stage.disconnect(widget.dragId);
            widget.dragId = 0;
        }
    }

    _clearWidgetContent(widget) {
        widget.spinner?.remove_all_transitions();
        widget.spinner = null;
        widget.bars = [];
        widget.timeLabel = null;
        widget.actor.destroy_all_children();
    }

    _setWidgetState(widget, state) {
        this._clearWidgetContent(widget);
        widget.state = state;
        const content = new St.BoxLayout({
            x_expand: true,
            y_expand: true,
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
            style: 'spacing: 10px;',
        });
        if (state === 'recording' || state === 'paused')
            this._fillControls(widget, content, state);
        else
            this._fillStatus(widget, content, state);
        widget.actor.add_child(content);
    }

    _fillControls(widget, content, state) {
        const bars = new St.BoxLayout({
            y_align: Clutter.ActorAlign.CENTER,
            style: `spacing: 3px; height: ${WIDGET_BAR_MAX_HEIGHT}px;`,
        });
        widget.bars = widget.heights.map(height => {
            const bar = new St.Widget({
                y_align: Clutter.ActorAlign.CENTER,
                style: 'width: 3px; border-radius: 2px; background-color: #e5e5e7;',
                height,
            });
            bars.add_child(bar);
            return bar;
        });
        widget.timeLabel = new St.Label({
            text: widget.elapsed,
            y_align: Clutter.ActorAlign.CENTER,
            style: 'min-width: 34px;',
        });
        content.add_child(bars);
        content.add_child(widget.timeLabel);
        if (state === 'paused')
            content.add_child(this._widgetButton('play', 'resume'));
        else
            content.add_child(this._widgetButton('pause', 'pause'));
        content.add_child(this._widgetButton('stop', 'stop'));
        content.add_child(this._widgetButton('cancel', 'cancel'));
    }

    _fillStatus(widget, content, state) {
        const icon = this._widgetIcon(state === 'done' ? 'check' : 'loader');
        content.add_child(icon);
        content.add_child(new St.Label({
            text: state === 'done' ? 'Copied' : 'Transcribing',
            y_align: Clutter.ActorAlign.CENTER,
        }));
        if (state === 'processing') {
            widget.spinner = icon;
            icon.set_pivot_point(0.5, 0.5);
            this._spinWidgetIcon(icon);
        }
    }

    _spinWidgetIcon(icon) {
        icon.rotation_angle_z = 0;
        icon.ease({
            rotation_angle_z: 360,
            duration: WIDGET_SPIN_MS,
            mode: Clutter.AnimationMode.LINEAR,
            onStopped: finished => {
                if (finished)
                    this._spinWidgetIcon(icon);
            },
        });
    }

    _widgetIcon(name) {
        const file = this.dir.get_child('icons').get_child(`recording-${name}.svg`);
        return new St.Icon({
            gicon: new Gio.FileIcon({file}),
            icon_size: 16,
            y_align: Clutter.ActorAlign.CENTER,
        });
    }

    _widgetButton(iconName, action) {
        const button = new St.Button({
            can_focus: false,
            reactive: true,
            track_hover: true,
            y_align: Clutter.ActorAlign.CENTER,
            style: 'width: 28px; height: 28px; border-radius: 14px;',
            child: this._widgetIcon(iconName),
        });
        button.connect('notify::hover', () => {
            button.style = `width: 28px; height: 28px; border-radius: 14px; ` +
                `background-color: ${button.hover ? WIDGET_BUTTON_HOVER : 'transparent'};`;
        });
        button.connect('clicked', () => this._emitRecordingWidgetAction(action));
        return button;
    }

    _updateWidgetBars(widget, level) {
        const clamped = Number.isFinite(level) ? Math.min(1, Math.max(0, level)) : 0;
        const range = WIDGET_BAR_MAX_HEIGHT - WIDGET_BAR_MIN_HEIGHT;
        widget.bars.forEach((bar, i) => {
            const target = WIDGET_BAR_MIN_HEIGHT + range * clamped * WIDGET_BAR_FACTORS[i];
            widget.heights[i] += (target - widget.heights[i]) * WIDGET_BAR_SMOOTHING;
            bar.height = Math.round(widget.heights[i]);
        });
    }

    _onAcceleratorActivated(id) {
        const action = this._grabbed.get(id);
        if (action !== undefined)
            this._dbus.emit_signal('ShortcutActivated', new GLib.Variant('(s)', [action]));
    }

    _ungrabAll() {
        for (const id of this._grabbed.keys())
            global.display.ungrab_accelerator(id);
        this._grabbed.clear();
    }

    _watchSender(sender) {
        this._unwatchSender();
        this._watchId = Gio.bus_watch_name(
            Gio.BusType.SESSION, sender, Gio.BusNameWatcherFlags.NONE, null,
            () => {
                this._unwatchSender();
                this._ungrabAll();
                this._removeRecordingWidget();
            });
    }

    _unwatchSender() {
        if (this._watchId) {
            Gio.bus_unwatch_name(this._watchId);
            this._watchId = 0;
        }
    }

    _findWindow(title) {
        return global.get_window_actors()
            .map(actor => actor.get_meta_window())
            .find(win => win.get_title() === title);
    }

    _placeOrWait(method, title, invocation, place) {
        try {
            this._pending.get(title)?.finish(false);
            const win = this._findWindow(title);
            if (win) {
                place(win);
                invocation.return_value(new GLib.Variant('(b)', [true]));
                return;
            }
            this._waitForWindow(title, place, invocation);
        } catch (e) {
            logError(e, `Promptheus: ${method} failed`);
            invocation.return_dbus_error('com.promptheus.Shell.Error', String(e));
        }
    }

    _placeNow(win, x, y, activate) {
        win.move_frame(true, x, y);
        if (activate)
            Main.activateWindow(win);
    }

    _anchorWindow(win, title, right, bottom, activate) {
        const previous = this._anchors.get(title);
        if (previous)
            this._releaseAnchor(previous);
        const anchor = {title, win, right, bottom, sizeId: 0, unmanagedId: 0, idleId: 0};
        anchor.sizeId = win.connect('size-changed',
            () => this._applyAnchor(anchor, 're-anchor on size-changed'));
        anchor.unmanagedId = win.connect('unmanaged', () => this._releaseAnchor(anchor));
        this._anchors.set(title, anchor);
        win.make_above();
        console.log(`Promptheus: make_above "${title}"`);
        this._applyAnchor(anchor, 'anchor');
        anchor.idleId = GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, () => {
            anchor.idleId = 0;
            this._applyAnchor(anchor, 're-anchor after placement');
            return GLib.SOURCE_REMOVE;
        });
        if (activate)
            Main.activateWindow(win);
    }

    _applyAnchor(anchor, reason) {
        const {title, win, right, bottom} = anchor;
        const before = win.get_frame_rect();
        const x = right - before.width;
        const y = bottom - before.height;
        const moved = before.x !== x || before.y !== y;
        if (moved)
            win.move_frame(true, x, y);
        const after = win.get_frame_rect();
        console.log(`Promptheus: ${reason} "${title}" right=${right} bottom=${bottom} ` +
            `moved=${moved} frame=${after.x},${after.y} ${after.width}x${after.height}`);
    }

    _releaseAnchor(anchor) {
        if (anchor.sizeId)
            anchor.win.disconnect(anchor.sizeId);
        if (anchor.unmanagedId)
            anchor.win.disconnect(anchor.unmanagedId);
        if (anchor.idleId)
            GLib.Source.remove(anchor.idleId);
        anchor.sizeId = 0;
        anchor.unmanagedId = 0;
        anchor.idleId = 0;
        if (this._anchors?.get(anchor.title) === anchor)
            this._anchors.delete(anchor.title);
        console.log(`Promptheus: anchor released "${anchor.title}"`);
    }

    _waitForWindow(title, place, invocation) {
        const display = global.display;
        const titleWatches = [];
        let createdId = 0;
        let timeoutId = 0;
        let actor = null;
        let frameId = 0;
        let done = false;

        const stopWatchingCreation = () => {
            if (createdId) {
                display.disconnect(createdId);
                createdId = 0;
            }
            for (const [win, id] of titleWatches)
                win.disconnect(id);
            titleWatches.length = 0;
        };

        const finish = placed => {
            if (done)
                return;
            done = true;
            stopWatchingCreation();
            if (timeoutId) {
                GLib.Source.remove(timeoutId);
                timeoutId = 0;
            }
            if (actor) {
                if (frameId)
                    actor.disconnect(frameId);
                actor.opacity = 255;
            }
            if (this._pending?.get(title) === pending)
                this._pending.delete(title);
            invocation.return_value(new GLib.Variant('(b)', [placed]));
        };
        const pending = {finish};

        const onFound = win => {
            stopWatchingCreation();
            actor = win.get_compositor_private();
            if (!actor) {
                place(win);
                finish(true);
                return;
            }
            actor.opacity = 0;
            frameId = actor.connect('first-frame', () => {
                actor.disconnect(frameId);
                frameId = 0;
                place(win);
                finish(true);
            });
        };

        createdId = display.connect('window-created', (_display, win) => {
            if (win.get_title() === title) {
                onFound(win);
                return;
            }
            const id = win.connect('notify::title', () => {
                if (win.get_title() === title)
                    onFound(win);
            });
            titleWatches.push([win, id]);
        });
        timeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, PLACE_WINDOW_TIMEOUT_MS, () => {
            timeoutId = 0;
            finish(false);
            return GLib.SOURCE_REMOVE;
        });
        this._pending.set(title, pending);
    }
}
