import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const PLACE_WINDOW_TIMEOUT_MS = 1000;
const OBJECT_PATH = '/com/promptheus/Shell';

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
    <signal name="ShortcutActivated">
      <arg type="s" name="action"/>
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
