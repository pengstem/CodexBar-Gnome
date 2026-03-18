import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';

import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

export default class CodexBarGnomePreferences extends ExtensionPreferences {
    fillPreferencesWindow(window) {
        const settings = this.getSettings();

        const page = new Adw.PreferencesPage({
            title: 'CodexBar-Gnome',
            icon_name: 'utilities-terminal-symbolic',
        });
        const group = new Adw.PreferencesGroup({
            title: 'Behavior',
            description: 'Configure how the Linux GNOME top bar extension talks to the backend.',
        });

        const backendRow = new Adw.ActionRow({
            title: 'Backend path',
            subtitle: 'Leave empty to use ~/.local/bin/codexbar-gnome-backend',
        });
        const backendEntry = new Gtk.Entry({
            text: settings.get_string('backend-path'),
            hexpand: true,
            valign: Gtk.Align.CENTER,
        });
        backendEntry.connect('changed', entry => {
            settings.set_string('backend-path', entry.get_text());
        });
        backendRow.add_suffix(backendEntry);
        backendRow.activatable_widget = backendEntry;
        group.add(backendRow);

        const providerRow = new Adw.ComboRow({
            title: 'Default provider',
            subtitle: 'The provider shown in the panel after GNOME Shell restarts.',
            model: Gtk.StringList.new(['codex', 'claude']),
        });
        providerRow.set_selected(settings.get_string('default-provider') === 'claude' ? 1 : 0);
        providerRow.connect('notify::selected', row => {
            settings.set_string('default-provider', row.get_selected() === 1 ? 'claude' : 'codex');
        });
        group.add(providerRow);

        const refreshRow = new Adw.ActionRow({
            title: 'Refresh interval',
            subtitle: 'Seconds between backend polls.',
        });
        const refreshAdjustment = new Gtk.Adjustment({
            lower: 15,
            upper: 900,
            step_increment: 15,
            page_increment: 60,
            value: settings.get_uint('refresh-interval'),
        });
        const refreshSpin = new Gtk.SpinButton({
            adjustment: refreshAdjustment,
            numeric: true,
            valign: Gtk.Align.CENTER,
        });
        refreshSpin.connect('value-changed', spin => {
            settings.set_uint('refresh-interval', spin.get_value_as_int());
        });
        refreshRow.add_suffix(refreshSpin);
        refreshRow.activatable_widget = refreshSpin;
        group.add(refreshRow);

        const showPercentRow = new Adw.SwitchRow({
            title: 'Show percentage in the panel',
            subtitle: 'Keep the remaining percentage visible next to the top bar icon.',
        });
        showPercentRow.set_active(settings.get_boolean('show-percentage'));
        showPercentRow.connect('notify::active', row => {
            settings.set_boolean('show-percentage', row.get_active());
        });
        group.add(showPercentRow);

        const staleRow = new Adw.SwitchRow({
            title: 'Keep the last successful value on error',
            subtitle: 'If the backend fails, continue showing the previous snapshot until refresh succeeds.',
        });
        staleRow.set_active(settings.get_boolean('keep-last-good-value'));
        staleRow.connect('notify::active', row => {
            settings.set_boolean('keep-last-good-value', row.get_active());
        });
        group.add(staleRow);

        page.add(group);
        window.add(page);
    }
}
