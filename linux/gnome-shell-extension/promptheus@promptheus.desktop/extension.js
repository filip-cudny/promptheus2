import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';
import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const PLACE_WINDOW_TIMEOUT_MS = 1000;
const OBJECT_PATH = '/com/promptheus/Shell';
const PROVIDER_MENU_TITLE = 'Promptheus Provider Menu';
const TOAST_DURATIONS_MS = {success: 2000, error: 4000, info: 2000, warning: 3000};
const TOAST_MARGIN = 20;
const TOAST_GAP = 14;
const TOAST_OPACITY = 204;
const TOAST_SHOW_ANIMATION_MS = 150;
const TOAST_HIDE_ANIMATION_MS = 150;
const WIDGET_WIDTH = 264;
const WIDGET_HEIGHT = 72;
const WIDGET_ROW_HEIGHT = WIDGET_HEIGHT / 2;
const WIDGET_RADIUS = 16;
const WIDGET_ITEM_ROW_HEIGHT = 22;
const WIDGET_ITEM_ROW_GAP = 4;
const WIDGET_ITEMS_ROW_PADDING = 10;
const WIDGET_ITEM_COLUMNS = 2;
const WIDGET_PADDING_X = 14;
const WIDGET_CHIP_WIDTH = (WIDGET_WIDTH - 2 * WIDGET_PADDING_X - (WIDGET_ITEM_COLUMNS - 1) * WIDGET_ITEM_ROW_GAP) / WIDGET_ITEM_COLUMNS;
const WIDGET_CHIP_PADDING_X = 6;
const WIDGET_PREVIEW_MAX_HEIGHT = 160;
const WIDGET_PREVIEW_GAP = 6;
const WIDGET_PREVIEW_DELAY_MS = 300;
const WIDGET_PREVIEW_PADDING = 8;
const WIDGET_DROPDOWN_ROW_HEIGHT = 28;
const WIDGET_DROPDOWN_PADDING = 4;
const WIDGET_DEFAULT_TARGET_LABEL = 'Dictation';
const WIDGET_BOTTOM_MARGIN = 24;
const WIDGET_STATES = ['recording', 'paused', 'processing', 'done'];
const WIDGET_BAR_MIN_HEIGHT = 4;
const WIDGET_BAR_MAX_HEIGHT = 28;
const WIDGET_BAR_MIN_SCALE = WIDGET_BAR_MIN_HEIGHT / WIDGET_BAR_MAX_HEIGHT;
const WIDGET_BAR_SCALE_EPSILON = 0.005;
const WIDGET_BAR_EASE_MS = 60;
const WIDGET_BAR_WIDTH = 2;
const WIDGET_BAR_SPACING = 3;
const WIDGET_TIME_WIDTH = 34;
const WIDGET_ROW_SPACING = 10;
const WIDGET_BUTTON_SIZE = 28;
const WIDGET_ACTION_SPACING = 4;
const WIDGET_ROW1_BUTTONS = 3;
const WIDGET_METER_WIDTH = WIDGET_WIDTH - 2 * WIDGET_PADDING_X - WIDGET_TIME_WIDTH - 2 * WIDGET_ROW_SPACING -
    WIDGET_ROW1_BUTTONS * WIDGET_BUTTON_SIZE - (WIDGET_ROW1_BUTTONS - 1) * WIDGET_ACTION_SPACING;
const WIDGET_BAR_COUNT = Math.floor(
    (WIDGET_METER_WIDTH + WIDGET_BAR_SPACING) / (WIDGET_BAR_WIDTH + WIDGET_BAR_SPACING));
const WIDGET_BAR_PATTERN = [0.5, 0.8, 0.65, 1, 0.7, 0.9, 0.55, 0.95, 0.75, 0.85, 0.6, 0.45, 0.8, 0.95, 0.6, 0.7, 1, 0.55];
const WIDGET_BAR_FACTORS = Array.from({length: WIDGET_BAR_COUNT},
    (_, i) => WIDGET_BAR_PATTERN[i % WIDGET_BAR_PATTERN.length]);
const WIDGET_WAVE_BAR_WIDTH = 2;
const WIDGET_WAVE_BAR_COUNT = Math.floor(
    (WIDGET_WIDTH - 2 * WIDGET_PADDING_X + WIDGET_BAR_SPACING) / (WIDGET_WAVE_BAR_WIDTH + WIDGET_BAR_SPACING));
const WIDGET_WAVE_HEIGHT = 24;
const WIDGET_WAVE_MIN_SCALE = WIDGET_BAR_MIN_HEIGHT / WIDGET_WAVE_HEIGHT;
const WIDGET_WAVE_AMPLITUDE = WIDGET_WAVE_HEIGHT / 3;
const WIDGET_WAVE_OPACITY = 150;
const WIDGET_DONE_ENTER_MS = 180;
const WIDGET_DONE_ENTER_SCALE = 0.85;
const WIDGET_WAVE_PERIOD_US = 1200 * 1000;
const WIDGET_WAVE_FRAME_MS = 33;
const WIDGET_WAVE_STEP = 2 * Math.PI / WIDGET_WAVE_BAR_COUNT;
const WIDGET_BUTTON_HOVER = '#3a3a3c';
const WIDGET_TARGET_PADDING_X = 8;
const WIDGET_DRAG_THRESHOLD_PX = 4;
const WIDGET_TARGET_ICONS = {'webview:claude': 'provider-claude', 'webview:chatgpt': 'provider-chatgpt'};
const WIDGET_TARGET_ICON_SIZE = 14;
const WIDGET_TARGET_ICON_GAP = 8;

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

function workAreaAt(x, y) {
    const areas = Main.layoutManager.monitors.map((_monitor, i) => Main.layoutManager.getWorkAreaForMonitor(i));
    return areas.find(area => pointInRect(x, y, area)) ?? areas[0];
}

function emptyWidgetModel() {
    return {
        targets: [],
        selectedId: '',
        selectedLabel: WIDGET_DEFAULT_TARGET_LABEL,
        locked: false,
        items: [],
        itemRows: 0,
        version: 0,
    };
}

function widgetItemsContentHeight(rows) {
    if (rows <= 0)
        return 0;
    return rows * WIDGET_ITEM_ROW_HEIGHT + (rows - 1) * WIDGET_ITEM_ROW_GAP;
}

function widgetItemsHeight(rows) {
    if (rows <= 0)
        return 0;
    return widgetItemsContentHeight(rows) + WIDGET_ITEMS_ROW_PADDING;
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
    <method name="PlaceWindowOnParent">
      <arg type="s" direction="in" name="title"/>
      <arg type="i" direction="in" name="dx"/>
      <arg type="i" direction="in" name="dy"/>
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
      <arg type="b" direction="in" name="monochromatic"/>
      <arg type="i" direction="out" name="x"/>
      <arg type="i" direction="out" name="y"/>
      <arg type="i" direction="out" name="work_x"/>
      <arg type="i" direction="out" name="work_y"/>
      <arg type="i" direction="out" name="work_width"/>
      <arg type="i" direction="out" name="work_height"/>
    </method>
    <method name="UpdateRecordingWidget">
      <arg type="s" direction="in" name="state"/>
      <arg type="d" direction="in" name="level"/>
      <arg type="u" direction="in" name="elapsed_ms"/>
    </method>
    <method name="UpdateRecordingWidgetModel">
      <arg type="a(ss)" direction="in" name="targets"/>
      <arg type="s" direction="in" name="selected_id"/>
      <arg type="s" direction="in" name="selected_label"/>
      <arg type="b" direction="in" name="target_locked"/>
      <arg type="a(ussss)" direction="in" name="items"/>
      <arg type="u" direction="in" name="item_rows"/>
      <arg type="t" direction="in" name="context_version"/>
      <arg type="i" direction="in" name="x"/>
      <arg type="i" direction="in" name="y"/>
    </method>
    <method name="HideRecordingWidget"/>
    <signal name="ShortcutActivated">
      <arg type="s" name="action"/>
    </signal>
    <signal name="RecordingWidgetAction">
      <arg type="s" name="action"/>
    </signal>
    <signal name="RecordingWidgetTargetSelected">
      <arg type="s" name="id"/>
    </signal>
    <signal name="RecordingWidgetItemRemoved">
      <arg type="u" name="index"/>
      <arg type="t" name="context_version"/>
    </signal>
    <signal name="RecordingWidgetMoved">
      <arg type="i" name="x"/>
      <arg type="i" name="y"/>
      <arg type="i" name="work_x"/>
      <arg type="i" name="work_y"/>
      <arg type="i" name="work_width"/>
      <arg type="i" name="work_height"/>
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
        this._mapId = global.window_manager.connect_after('map', (shellwm, actor) => this._pivotProviderMenuAtTop(actor));
        this._destroyId = global.window_manager.connect_after('destroy', (shellwm, actor) => this._pivotProviderMenuAtTop(actor));
        this._dbus.emit_signal('Ready', null);
    }

    _pivotProviderMenuAtTop(actor) {
        if (actor.meta_window?.get_title() === PROVIDER_MENU_TITLE)
            actor.set_pivot_point(0.5, 0);
    }

    disable() {
        if (this._acceleratorId) {
            global.display.disconnect(this._acceleratorId);
            this._acceleratorId = 0;
        }
        if (this._mapId) {
            global.window_manager.disconnect(this._mapId);
            this._mapId = 0;
        }
        if (this._destroyId) {
            global.window_manager.disconnect(this._destroyId);
            this._destroyId = 0;
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

    PlaceWindowOnParentAsync(params, invocation) {
        const [title, dx, dy, activate] = params;
        this._placeOrWait('PlaceWindowOnParent', title, invocation,
            win => this._placeOnParent(win, title, dx, dy, activate));
    }

    GetFocusedWmClass() {
        return global.display.focus_window?.get_wm_class() ?? '';
    }

    ShowToast(level, title, message, monochromatic) {
        const duration = TOAST_DURATIONS_MS[level];
        if (duration === undefined)
            throw new Error(`Unknown toast level: ${level}`);
        const actor = this._buildToast(level, title, message, monochromatic);
        if (this._toasts.length === 0) {
            this._toastMonitor = global.display.get_current_monitor();
            global.compositor.disable_unredirect();
        }
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
        if (this._toasts.length === 0)
            global.compositor.enable_unredirect();
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
        if (toasts.length > 0)
            global.compositor.enable_unredirect();
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

    ShowRecordingWidget(x, y, hasPosition, monochromatic) {
        if (!this._widget) {
            this._widget = this._buildRecordingWidget();
            global.compositor.disable_unredirect();
        }
        const widget = this._widget;
        widget.monochromatic = monochromatic;
        widget.scales.fill(WIDGET_BAR_MIN_SCALE);
        widget.elapsed = formatElapsed(0);
        widget.model = emptyWidgetModel();
        this._closeDropdown(widget);
        this._fillItems(widget);
        this._setWidgetState(widget, 'recording');
        const monitors = Main.layoutManager.monitors;
        const workAreas = monitors.map((_monitor, i) => Main.layoutManager.getWorkAreaForMonitor(i));
        const pointerArea = Main.layoutManager.getWorkAreaForMonitor(global.display.get_current_monitor());
        const position = resolveWidgetPosition(hasPosition, x, y, workAreas, pointerArea);
        widget.actor.set_position(position.x, position.y);
        this._applyWidgetSize(widget);
        console.log(`Promptheus: recording widget shown at ${position.x},${position.y} ` +
            `stored=${position.stored}`);
        const area = workAreaAt(position.x, position.y);
        return [position.x, position.y, area.x, area.y, area.width, area.height];
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
            this._easeWidgetBars(widget, level);
        if (widget.timeLabel && widget.timeLabel.text !== widget.elapsed)
            widget.timeLabel.text = widget.elapsed;
    }

    UpdateRecordingWidgetModel(targets, selectedId, selectedLabel, locked, items, itemRows, version, x, y) {
        const widget = this._widget;
        if (!widget)
            throw new Error('Recording widget is not shown');
        widget.model = {targets, selectedId, selectedLabel, locked, items, itemRows, version};
        widget.actor.set_position(x, y);
        this._fillRow2(widget);
        this._fillItems(widget);
        this._applyWidgetSize(widget);
        if (widget.dropdown)
            this._openDropdown(widget);
    }

    HideRecordingWidget() {
        this._removeRecordingWidget();
    }

    _removeRecordingWidget() {
        const widget = this._widget;
        if (!widget)
            return;
        this._widget = null;
        global.compositor.enable_unredirect();
        this._endWidgetDrag(widget);
        this._closeDropdown(widget);
        this._closePreview(widget);
        this._clearWidgetContent(widget);
        widget.actor.remove_all_transitions();
        Main.layoutManager.removeChrome(widget.actor);
        widget.actor.destroy();
        console.log('Promptheus: recording widget hidden');
    }

    _emitRecordingWidgetAction(action) {
        this._dbus.emit_signal('RecordingWidgetAction', new GLib.Variant('(s)', [action]));
    }

    _emitRecordingWidgetTargetSelected(id) {
        this._dbus.emit_signal('RecordingWidgetTargetSelected', new GLib.Variant('(s)', [id]));
    }

    _emitRecordingWidgetItemRemoved(index, version) {
        this._dbus.emit_signal('RecordingWidgetItemRemoved', new GLib.Variant('(ut)', [index, version]));
    }

    _widgetRow(height) {
        return new St.BoxLayout({style: `height: ${height}px; padding: 0 ${WIDGET_PADDING_X}px;`});
    }

    _buildRecordingWidget() {
        const actor = new St.BoxLayout({
            vertical: true,
            style: `border-radius: ${WIDGET_RADIUS}px; background-color: rgba(28,28,30,0.92); ` +
                'color: #e5e5e7; font-family: "Noto Sans", sans-serif; font-size: 13px; ' +
                'box-shadow: inset 0 0 0 1px rgba(255,255,255,0.12);',
            reactive: true,
            can_focus: false,
            track_hover: false,
        });
        const widget = {
            actor,
            state: null,
            model: emptyWidgetModel(),
            itemsRow: this._widgetRow(0),
            preview: null,
            previewId: 0,
            row1: this._widgetRow(WIDGET_ROW_HEIGHT),
            row2: this._widgetRow(WIDGET_ROW_HEIGHT),
            dropdown: null,
            elapsed: formatElapsed(0),
            scales: WIDGET_BAR_FACTORS.map(() => WIDGET_BAR_MIN_SCALE),
            bars: [],
            timeLabel: null,
            waveBars: [],
            waveStart: 0,
            waveId: 0,
            dragId: 0,
        };
        widget.itemsRow.visible = false;
        widget.row2.style += ' spacing: 6px;';
        actor.add_child(widget.itemsRow);
        actor.add_child(widget.row1);
        actor.add_child(widget.row2);
        actor.connect('button-press-event', (_actor, event) => this._beginWidgetDrag(widget, event));
        Main.layoutManager.addTopChrome(actor);
        return widget;
    }

    _beginWidgetDrag(widget, event) {
        if (event.get_button() !== Clutter.BUTTON_PRIMARY)
            return Clutter.EVENT_PROPAGATE;
        if (this._isInWidgetButton(widget, global.stage.get_event_actor(event)))
            return Clutter.EVENT_PROPAGATE;
        this._endWidgetDrag(widget);
        const [pressX, pressY] = event.get_coords();
        const offsetX = pressX - widget.actor.x;
        const offsetY = pressY - widget.actor.y;
        let dragging = false;
        widget.dragId = global.stage.connect('captured-event', (_stage, e) => {
            const type = e.type();
            const primaryLifted = type === Clutter.EventType.MOTION &&
                !(e.get_state() & Clutter.ModifierType.BUTTON1_MASK);
            if (type === Clutter.EventType.BUTTON_RELEASE || primaryLifted) {
                this._endWidgetDrag(widget);
                if (!dragging)
                    return Clutter.EVENT_PROPAGATE;
                this._emitRecordingWidgetMoved(widget);
                return Clutter.EVENT_STOP;
            }
            if (type === Clutter.EventType.MOTION) {
                const [x, y] = e.get_coords();
                if (!dragging && Math.hypot(x - pressX, y - pressY) <= WIDGET_DRAG_THRESHOLD_PX)
                    return Clutter.EVENT_PROPAGATE;
                if (!dragging) {
                    this._closeDropdown(widget);
                    this._closePreview(widget);
                }
                dragging = true;
                widget.actor.set_position(Math.round(x - offsetX), Math.round(y - offsetY));
                return Clutter.EVENT_STOP;
            }
            return Clutter.EVENT_PROPAGATE;
        });
        return Clutter.EVENT_STOP;
    }

    _isInWidgetButton(widget, actor) {
        for (let current = actor; current && current !== widget.actor; current = current.get_parent()) {
            if (current instanceof St.Button || current.isWidgetChip)
                return true;
        }
        return false;
    }

    _emitRecordingWidgetMoved(widget) {
        const x = Math.round(widget.actor.x);
        const y = Math.round(widget.actor.y);
        console.log(`Promptheus: recording widget dragged to ${x},${y}`);
        const area = workAreaAt(x, y);
        this._dbus.emit_signal('RecordingWidgetMoved',
            new GLib.Variant('(iiiiii)', [x, y, area.x, area.y, area.width, area.height]));
    }

    _endWidgetDrag(widget) {
        if (widget.dragId) {
            global.stage.disconnect(widget.dragId);
            widget.dragId = 0;
        }
    }

    _clearWidgetContent(widget) {
        widget.bars.forEach(bar => bar.remove_all_transitions());
        this._stopWidgetWave(widget);
        widget.bars = [];
        widget.timeLabel = null;
        widget.row1.destroy_all_children();
    }

    _applyWidgetSize(widget) {
        const height = WIDGET_HEIGHT + widgetItemsHeight(widget.model.itemRows);
        widget.actor.set_size(WIDGET_WIDTH, height);
    }

    _setWidgetState(widget, state) {
        this._clearWidgetContent(widget);
        widget.state = state;
        const content = new St.BoxLayout({
            x_expand: true,
            y_expand: true,
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.CENTER,
            style: `spacing: ${WIDGET_ROW_SPACING}px;`,
        });
        if (state === 'recording' || state === 'paused') {
            content.x_align = Clutter.ActorAlign.FILL;
            this._fillControls(widget, content, state);
        }
        else if (state === 'processing')
            this._fillWave(widget, content);
        else
            this._fillDone(widget, content);
        widget.row1.add_child(content);
        if (state === 'done') {
            content.set_pivot_point(0.5, 0.5);
            content.set_scale(WIDGET_DONE_ENTER_SCALE, WIDGET_DONE_ENTER_SCALE);
            content.opacity = 0;
            content.ease({
                scale_x: 1,
                scale_y: 1,
                opacity: 255,
                duration: WIDGET_DONE_ENTER_MS,
                mode: Clutter.AnimationMode.EASE_OUT_CUBIC,
            });
        }
        this._fillRow2(widget);
    }

    _fillControls(widget, content, state) {
        const bars = this._widgetBarRow();
        if (state === 'paused')
            bars.opacity = WIDGET_WAVE_OPACITY;
        widget.bars = widget.scales.map(scale => this._widgetBar(bars, scale));
        widget.timeLabel = new St.Label({
            text: widget.elapsed,
            y_align: Clutter.ActorAlign.CENTER,
            style: `min-width: ${WIDGET_TIME_WIDTH}px;`,
        });
        content.add_child(bars);
        content.add_child(widget.timeLabel);
        content.add_child(this._widgetActions(
            state === 'paused'
                ? this._widgetActionButton('play', 'resume')
                : this._widgetActionButton('pause', 'pause'),
            this._widgetActionButton('cancel', 'cancel', 'cancel-hover'),
            this._widgetActionButton(widget.monochromatic ? 'check-mono' : 'check', 'stop')));
    }

    _widgetActions(...buttons) {
        const actions = new St.BoxLayout({
            x_expand: true,
            x_align: Clutter.ActorAlign.END,
            style: `spacing: ${WIDGET_ACTION_SPACING}px;`,
        });
        buttons.forEach(button => actions.add_child(button));
        return actions;
    }

    _widgetBarRow(spacing = WIDGET_BAR_SPACING, height = WIDGET_BAR_MAX_HEIGHT) {
        return new St.BoxLayout({
            y_align: Clutter.ActorAlign.CENTER,
            style: `spacing: ${spacing}px; height: ${height}px;`,
        });
    }

    _widgetBar(row, scale, width = WIDGET_BAR_WIDTH, height = WIDGET_BAR_MAX_HEIGHT) {
        const bar = new St.Widget({
            y_align: Clutter.ActorAlign.CENTER,
            style: `width: ${width}px; border-radius: 2px; background-color: #e5e5e7;`,
            height,
            scale_y: scale,
        });
        bar.set_pivot_point(0.5, 0.5);
        row.add_child(bar);
        return bar;
    }

    _fillWave(widget, content) {
        const row = this._widgetBarRow(WIDGET_BAR_SPACING, WIDGET_WAVE_HEIGHT);
        row.opacity = WIDGET_WAVE_OPACITY;
        widget.waveBars = Array.from({length: WIDGET_WAVE_BAR_COUNT},
            () => this._widgetBar(row, WIDGET_WAVE_MIN_SCALE, WIDGET_WAVE_BAR_WIDTH, WIDGET_WAVE_HEIGHT));
        content.add_child(row);
        widget.waveStart = GLib.get_monotonic_time();
        this._updateWidgetWave(widget);
        widget.waveId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, WIDGET_WAVE_FRAME_MS, () => {
            this._updateWidgetWave(widget);
            return GLib.SOURCE_CONTINUE;
        });
    }

    _updateWidgetWave(widget) {
        const elapsed = GLib.get_monotonic_time() - widget.waveStart;
        const phase = 2 * Math.PI * (elapsed % WIDGET_WAVE_PERIOD_US) / WIDGET_WAVE_PERIOD_US;
        widget.waveBars.forEach((bar, i) => {
            const wave = 0.5 + 0.5 * Math.sin(phase - i * WIDGET_WAVE_STEP);
            bar.scale_y = (WIDGET_BAR_MIN_HEIGHT + WIDGET_WAVE_AMPLITUDE * wave) / WIDGET_WAVE_HEIGHT;
        });
    }

    _stopWidgetWave(widget) {
        if (widget.waveId) {
            GLib.Source.remove(widget.waveId);
            widget.waveId = 0;
        }
        widget.waveBars = [];
    }

    _fillDone(widget, content) {
        content.add_child(this._widgetIcon(widget.monochromatic ? 'check-mono' : 'check'));
        content.add_child(new St.Label({
            text: 'Copied',
            y_align: Clutter.ActorAlign.CENTER,
        }));
    }

    _widgetGicon(name) {
        return new Gio.FileIcon({file: this.dir.get_child('icons').get_child(`recording-${name}.svg`)});
    }

    _widgetIcon(name, size = 16) {
        return new St.Icon({
            gicon: this._widgetGicon(name),
            icon_size: size,
            y_align: Clutter.ActorAlign.CENTER,
        });
    }

    _widgetButton(child, onClick,
        baseStyle = `width: ${WIDGET_BUTTON_SIZE}px; height: ${WIDGET_BUTTON_SIZE}px; border-radius: ${WIDGET_BUTTON_SIZE / 2}px;`) {
        const button = new St.Button({
            can_focus: false,
            reactive: true,
            track_hover: true,
            y_align: Clutter.ActorAlign.CENTER,
            style: baseStyle,
            child,
        });
        button.connect('notify::hover', () => {
            button.style = `${baseStyle} background-color: ${button.hover ? WIDGET_BUTTON_HOVER : 'transparent'};`;
        });
        button.connect('clicked', onClick);
        return button;
    }

    _widgetActionButton(iconName, action, hoverIconName = iconName) {
        const icon = this._widgetIcon(iconName);
        const button = this._widgetButton(icon, () => this._emitRecordingWidgetAction(action));
        if (hoverIconName !== iconName) {
            button.connect('notify::hover', () => {
                icon.gicon = this._widgetGicon(button.hover ? hoverIconName : iconName);
            });
        }
        return button;
    }

    _fillRow2(widget) {
        widget.row2.destroy_all_children();
        const {selectedLabel, locked} = widget.model;
        const interactive = !locked && widget.state !== 'done';
        if (!interactive) {
            this._closeDropdown(widget);
            widget.row2.style = `height: ${WIDGET_ROW_HEIGHT}px; padding: 0 ${WIDGET_PADDING_X}px;`;
            widget.row2.add_child(new St.Label({
                text: selectedLabel,
                x_expand: true,
                x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.CENTER,
                opacity: WIDGET_WAVE_OPACITY,
            }));
            return;
        }
        widget.row2.style = `height: ${WIDGET_ROW_HEIGHT}px; spacing: ${WIDGET_ROW_SPACING}px; ` +
            `padding: 0 ${WIDGET_PADDING_X}px 0 ${WIDGET_PADDING_X - WIDGET_TARGET_PADDING_X}px;`;
        const content = new St.BoxLayout({style: 'spacing: 4px;'});
        content.opacity = WIDGET_WAVE_OPACITY;
        content.add_child(new St.Label({text: selectedLabel, y_align: Clutter.ActorAlign.CENTER}));
        content.add_child(this._widgetIcon('chevrons-up-down', 14));
        const target = this._widgetButton(content, () => this._toggleDropdown(widget),
            `height: 28px; border-radius: 14px; padding: 0 ${WIDGET_TARGET_PADDING_X}px;`);
        widget.row2.add_child(target);
        widget.row2.add_child(this._widgetActions(
            this._widgetActionButton('paste', 'paste'),
            this._widgetActionButton('chat', 'stop_chat')));
    }

    _fillItems(widget) {
        const {items, locked, itemRows, version} = widget.model;
        this._closePreview(widget);
        widget.itemsRow.destroy_all_children();
        widget.itemsRow.visible = items.length > 0;
        widget.itemsRow.style = `height: ${widgetItemsContentHeight(itemRows)}px; padding: ${WIDGET_ITEMS_ROW_PADDING / 2}px ${WIDGET_PADDING_X}px;`;
        const grid = new St.BoxLayout({vertical: true, style: `spacing: ${WIDGET_ITEM_ROW_GAP}px;`});
        let line = null;
        items.forEach(([index, kind, label, preview, imagePath], position) => {
            if (position % WIDGET_ITEM_COLUMNS === 0) {
                line = new St.BoxLayout({style: `spacing: ${WIDGET_ITEM_ROW_GAP}px;`});
                grid.add_child(line);
            }
            line.add_child(this._widgetChip(widget, kind, label, preview, imagePath,
                locked ? null : () => this._emitRecordingWidgetItemRemoved(index, version)));
        });
        const scroll = new St.ScrollView({
            overlay_scrollbars: true,
            hscrollbar_policy: St.PolicyType.NEVER,
            vscrollbar_policy: St.PolicyType.AUTOMATIC,
            x_expand: true,
            y_expand: true,
        });
        scroll.set_child(grid);
        widget.itemsRow.add_child(scroll);
    }

    _widgetChip(widget, kind, label, text, imagePath, onRemove) {
        const chip = new St.BoxLayout({
            reactive: true,
            track_hover: true,
            y_align: Clutter.ActorAlign.CENTER,
            style: `width: ${WIDGET_CHIP_WIDTH - 2 * WIDGET_CHIP_PADDING_X}px; height: ${WIDGET_ITEM_ROW_HEIGHT}px; padding: 0 ${WIDGET_CHIP_PADDING_X}px; spacing: 4px; border-radius: 11px; ` +
                'background-color: rgba(255,255,255,0.12); font-size: 11px;',
        });
        chip.isWidgetChip = true;
        chip.add_child(this._widgetIcon(`item-${kind}`, 12));
        const labelActor = new St.Label({text: label, x_expand: true, y_align: Clutter.ActorAlign.CENTER});
        labelActor.clutter_text.ellipsize = Pango.EllipsizeMode.END;
        chip.add_child(labelActor);
        if (onRemove) {
            chip.add_child(this._widgetButton(this._widgetIcon('item-remove', 12), onRemove,
                'width: 16px; height: 16px; border-radius: 8px;'));
        }
        chip.connect('notify::hover', () => {
            if (chip.hover)
                this._schedulePreview(widget, chip, text, imagePath);
            else
                this._closePreview(widget);
        });
        return chip;
    }

    _schedulePreview(widget, chip, text, imagePath) {
        this._closePreview(widget);
        if (widget.dropdown || (!text && !imagePath))
            return;
        widget.previewId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, WIDGET_PREVIEW_DELAY_MS, () => {
            widget.previewId = 0;
            this._showPreview(widget, text, imagePath);
            return GLib.SOURCE_REMOVE;
        });
    }

    _showPreview(widget, text, imagePath) {
        if (widget.dropdown)
            return;
        const style = 'background-color: #252629; border: 1px solid rgba(255,255,255,0.12); border-radius: 6px; ' +
            'color: #e5e5e7; font-family: "Noto Sans", sans-serif; font-size: 12px;';
        let popover;
        if (imagePath) {
            popover = new St.Widget({
                reactive: false,
                style: `${style} background-image: url("file://${imagePath}"); background-size: contain; ` +
                    'background-position: center; background-repeat: no-repeat;',
            });
            popover.set_size(WIDGET_WIDTH, WIDGET_PREVIEW_MAX_HEIGHT);
        } else {
            const label = new St.Label({text});
            label.clutter_text.line_wrap = true;
            label.clutter_text.line_wrap_mode = Pango.WrapMode.WORD_CHAR;
            label.clutter_text.ellipsize = Pango.EllipsizeMode.NONE;
            popover = new St.Bin({
                reactive: false,
                style: `${style} padding: ${WIDGET_PREVIEW_PADDING}px;`,
                child: label,
                clip_to_allocation: true,
            });
            Main.layoutManager.uiGroup.add_child(popover);
            const innerWidth = WIDGET_WIDTH - 2 * WIDGET_PREVIEW_PADDING;
            const [, natural] = label.get_preferred_height(innerWidth);
            popover.set_size(WIDGET_WIDTH,
                Math.min(natural + 2 * WIDGET_PREVIEW_PADDING, WIDGET_PREVIEW_MAX_HEIGHT));
        }
        const {actor} = widget;
        const y = this._dropdownAbove(widget)
            ? actor.y - popover.height - WIDGET_PREVIEW_GAP
            : actor.y + actor.height + WIDGET_PREVIEW_GAP;
        popover.set_position(actor.x, y);
        if (!popover.get_parent())
            Main.layoutManager.uiGroup.add_child(popover);
        widget.preview = popover;
    }

    _closePreview(widget) {
        if (widget.previewId) {
            GLib.Source.remove(widget.previewId);
            widget.previewId = 0;
        }
        if (widget.preview) {
            widget.preview.destroy();
            widget.preview = null;
        }
    }

    _dropdownAbove(widget) {
        const {actor} = widget;
        const centreY = actor.y + actor.height / 2;
        const area = workAreaAt(actor.x + actor.width / 2, centreY);
        return centreY > area.y + area.height / 2;
    }

    _toggleDropdown(widget) {
        if (widget.dropdown)
            this._closeDropdown(widget);
        else
            this._openDropdown(widget);
    }

    _openDropdown(widget) {
        this._closeDropdown(widget);
        this._closePreview(widget);
        const {targets, selectedId} = widget.model;
        if (targets.length === 0)
            return;
        const height = targets.length * WIDGET_DROPDOWN_ROW_HEIGHT + 2 * WIDGET_DROPDOWN_PADDING;
        const dropdown = new St.BoxLayout({
            vertical: true,
            reactive: true,
            can_focus: false,
            style: `padding: ${WIDGET_DROPDOWN_PADDING}px 0; border-radius: ${WIDGET_RADIUS}px; ` +
                'background-color: rgba(28,28,30,0.92); color: #e5e5e7; ' +
                'font-family: "Noto Sans", sans-serif; font-size: 13px;',
        });
        dropdown.set_size(WIDGET_WIDTH, height);
        const rowRadius = WIDGET_RADIUS - WIDGET_DROPDOWN_PADDING;
        targets.forEach(([id, label], index) => {
            const weight = id === selectedId ? 'font-weight: 700;' : '';
            const top = index === 0 ? rowRadius : 0;
            const bottom = index === targets.length - 1 ? rowRadius : 0;
            dropdown.add_child(this._widgetButton(this._dropdownRowContent(id, label),
                () => {
                    this._emitRecordingWidgetTargetSelected(id);
                    this._closeDropdown(widget);
                },
                `height: ${WIDGET_DROPDOWN_ROW_HEIGHT}px; padding: 0 ${WIDGET_PADDING_X}px; ` +
                `border-radius: ${top}px ${top}px ${bottom}px ${bottom}px; ${weight}`));
        });
        const {actor} = widget;
        dropdown.set_position(actor.x, this._dropdownAbove(widget) ? actor.y - height : actor.y + actor.height);
        Main.layoutManager.addTopChrome(dropdown);
        widget.dropdown = dropdown;
    }

    _dropdownRowContent(id, label) {
        const text = new St.Label({
            text: label,
            x_expand: true,
            x_align: Clutter.ActorAlign.START,
            y_align: Clutter.ActorAlign.CENTER,
        });
        const icon = WIDGET_TARGET_ICONS[id];
        if (!icon)
            return text;
        const content = new St.BoxLayout({
            x_expand: true,
            x_align: Clutter.ActorAlign.START,
            style: `spacing: ${WIDGET_TARGET_ICON_GAP}px;`,
        });
        content.add_child(this._widgetIcon(icon, WIDGET_TARGET_ICON_SIZE));
        content.add_child(text);
        return content;
    }

    _closeDropdown(widget) {
        if (!widget.dropdown)
            return;
        Main.layoutManager.removeChrome(widget.dropdown);
        widget.dropdown.destroy();
        widget.dropdown = null;
    }

    _easeWidgetBars(widget, level) {
        const clamped = Number.isFinite(level) ? Math.min(1, Math.max(0, level)) : 0;
        const range = 1 - WIDGET_BAR_MIN_SCALE;
        widget.bars.forEach((bar, i) => {
            const scale = WIDGET_BAR_MIN_SCALE + range * clamped * WIDGET_BAR_FACTORS[i];
            if (Math.abs(scale - widget.scales[i]) < WIDGET_BAR_SCALE_EPSILON)
                return;
            widget.scales[i] = scale;
            bar.ease({
                scale_y: scale,
                duration: WIDGET_BAR_EASE_MS,
                mode: Clutter.AnimationMode.LINEAR,
            });
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
                const placed = place(win) !== false;
                invocation.return_value(new GLib.Variant('(b)', [placed]));
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

    _placeOnParent(win, title, dx, dy, activate) {
        const parent = win.get_transient_for();
        if (!parent) {
            console.warn(`Promptheus: PlaceWindowOnParent "${title}" has no transient parent`);
            return false;
        }
        const p = parent.get_frame_rect();
        win.move_frame(true, p.x + dx, p.y + dy);
        if (activate)
            Main.activateWindow(win);
        const f = win.get_frame_rect();
        console.log(`Promptheus: place on parent "${title}" parent=${p.x},${p.y} frame=${f.x},${f.y} ${f.width}x${f.height}`);
        return true;
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
                finish(place(win) !== false);
                return;
            }
            actor.opacity = 0;
            frameId = actor.connect('first-frame', () => {
                actor.disconnect(frameId);
                frameId = 0;
                finish(place(win) !== false);
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
