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
        this._pending = null;
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
        try {
            const [title, x, y, activate] = params;
            this._pending.get(title)?.finish(false);
            const win = this._findWindow(title);
            if (win) {
                this._placeNow(win, x, y, activate);
                invocation.return_value(new GLib.Variant('(b)', [true]));
                return;
            }
            this._waitForWindow(title, x, y, activate, invocation);
        } catch (e) {
            logError(e, 'Promptheus: PlaceWindow failed');
            invocation.return_dbus_error('com.promptheus.Shell.Error', String(e));
        }
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

    _placeNow(win, x, y, activate) {
        win.move_frame(true, x, y);
        if (activate)
            Main.activateWindow(win);
    }

    _waitForWindow(title, x, y, activate, invocation) {
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
                this._placeNow(win, x, y, activate);
                finish(true);
                return;
            }
            actor.opacity = 0;
            frameId = actor.connect('first-frame', () => {
                actor.disconnect(frameId);
                frameId = 0;
                this._placeNow(win, x, y, activate);
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
