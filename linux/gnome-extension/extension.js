import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';
import Clutter from 'gi://Clutter';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const DEFAULT_BACKEND_PATH = `${GLib.get_home_dir()}/.local/bin/codexbar-gnome-backend`;
const DEFAULT_CONFIG_DIR = `${GLib.get_home_dir()}/.config/codexbar-gnome`;
const PANEL_METER_WIDTH = 32;
const MENU_METER_WIDTH = 244;

const PROVIDER_META = {
    codex: {
        label: 'Codex',
        badge: 'CX',
        accent: '#74c8ff',
        accentSoft: 'rgba(116, 200, 255, 0.18)',
        accentBorder: 'rgba(116, 200, 255, 0.34)',
    },
    claude: {
        label: 'Claude',
        badge: 'CL',
        accent: '#ffb467',
        accentSoft: 'rgba(255, 180, 103, 0.18)',
        accentBorder: 'rgba(255, 180, 103, 0.34)',
    },
};

const STATUS_META = {
    none: {label: 'Operational', color: '#7ee787'},
    minor: {label: 'Minor incident', color: '#f2cc60'},
    major: {label: 'Major incident', color: '#ff8e72'},
    critical: {label: 'Critical incident', color: '#ff6b81'},
    maintenance: {label: 'Maintenance', color: '#8ea6ff'},
    unknown: {label: 'Unknown', color: '#9aa4b2'},
};

const CodexBarGnomeIndicator = GObject.registerClass(
class CodexBarGnomeIndicator extends PanelMenu.Button {
    _init(extension) {
        super._init(0.0, 'CodexBar-Gnome');

        this._extension = extension;
        this._settings = extension.getSettings();
        this._settingsSignals = [];
        this._timeoutId = 0;
        this._refreshInFlight = false;
        this._lastPayload = null;
        this._lastError = null;

        this._buildPanel();
        this._buildMenu();
        this._connectSettings();
        this._scheduleRefresh();
        this.refresh();
    }

    destroy() {
        this._disconnectSettings();
        if (this._timeoutId) {
            GLib.Source.remove(this._timeoutId);
            this._timeoutId = 0;
        }
        super.destroy();
    }

    _buildPanel() {
        this._panelBox = new St.BoxLayout({
            style_class: 'panel-status-menu-box codexbar-panel-box',
        });

        this._panelMeters = new St.BoxLayout({
            vertical: true,
            style_class: 'codexbar-panel-meters',
        });
        this._panelSessionMeter = this._createMeter('codexbar-panel-meter codexbar-panel-meter-primary');
        this._panelWeeklyMeter = this._createMeter('codexbar-panel-meter codexbar-panel-meter-secondary');
        this._panelMeters.add_child(this._panelSessionMeter.track);
        this._panelMeters.add_child(this._panelWeeklyMeter.track);

        this._panelBox.add_child(this._panelMeters);
        this.add_child(this._panelBox);
    }

    _buildMenu() {
        this._cardItem = new PopupMenu.PopupBaseMenuItem({
            reactive: false,
            can_focus: false,
        });
        this._cardItem.add_style_class_name('codexbar-card-item');

        this._card = new St.BoxLayout({
            vertical: true,
            style_class: 'codexbar-card',
            x_expand: true,
        });
        this._cardItem.add_child(this._card);

        this._header = this._buildHeader();
        this._errorBanner = this._buildErrorBanner();
        this._usageSection = this._buildUsageSection();
        this._creditsSection = this._buildCreditsSection();
        this._factsSection = this._buildFactsSection();

        this._card.add_child(this._header.box);
        this._card.add_child(this._errorBanner.box);
        this._card.add_child(this._createDivider());
        this._card.add_child(this._usageSection.box);
        this._card.add_child(this._creditsSection.box);
        this._card.add_child(this._factsSection.box);

        this.menu.addMenuItem(this._cardItem);
        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        this.menu.addMenuItem(this._buildActionItem());
    }

    _buildHeader() {
        const box = new St.BoxLayout({
            vertical: true,
            style_class: 'codexbar-card-header',
        });

        const topRow = new St.BoxLayout({
            style_class: 'codexbar-card-header-top',
        });
        const identityColumn = new St.BoxLayout({
            vertical: true,
            x_expand: true,
            style_class: 'codexbar-card-identity-column',
        });
        const titleRow = new St.BoxLayout({
            style_class: 'codexbar-card-title-row',
        });

        const chip = new St.Label({
            text: 'CX',
            style_class: 'codexbar-provider-chip',
        });
        const providerName = new St.Label({
            text: 'Codex',
            style_class: 'codexbar-provider-name',
        });
        titleRow.add_child(chip);
        titleRow.add_child(providerName);

        const identity = new St.Label({
            text: 'Waiting for provider data…',
            style_class: 'codexbar-card-identity',
        });
        const subtitle = new St.Label({
            text: 'Backend is starting',
            style_class: 'codexbar-card-subtitle',
        });

        identityColumn.add_child(titleRow);
        identityColumn.add_child(identity);
        identityColumn.add_child(subtitle);

        const heroValue = new St.Label({
            text: '--%',
            style_class: 'codexbar-card-hero',
            x_align: Clutter.ActorAlign.END,
        });

        topRow.add_child(identityColumn);
        topRow.add_child(heroValue);
        box.add_child(topRow);

        return {
            box,
            chip,
            providerName,
            identity,
            subtitle,
            heroValue,
        };
    }

    _buildErrorBanner() {
        const box = new St.BoxLayout({
            vertical: true,
            style_class: 'codexbar-error-banner',
        });
        const title = new St.Label({
            text: 'Backend error',
            style_class: 'codexbar-error-title',
        });
        const body = new St.Label({
            text: '',
            style_class: 'codexbar-error-body',
        });
        box.add_child(title);
        box.add_child(body);
        box.visible = false;
        return {box, body};
    }

    _buildUsageSection() {
        const box = new St.BoxLayout({
            vertical: true,
            style_class: 'codexbar-section',
        });
        const title = new St.Label({
            text: 'Usage',
            style_class: 'codexbar-section-title',
        });
        box.add_child(title);

        const session = this._createMetricRow('Session');
        const weekly = this._createMetricRow('Weekly');
        const tertiary = this._createMetricRow('Model cap');

        box.add_child(session.item);
        box.add_child(weekly.item);
        box.add_child(tertiary.item);

        return {box, session, weekly, tertiary};
    }

    _buildCreditsSection() {
        const box = new St.BoxLayout({
            vertical: true,
            style_class: 'codexbar-credits-section',
        });

        const titleRow = new St.BoxLayout({
            style_class: 'codexbar-credits-header',
        });
        const title = new St.Label({
            text: 'Credits',
            style_class: 'codexbar-section-title',
        });
        const value = new St.Label({
            text: '--',
            style_class: 'codexbar-credits-value',
        });
        titleRow.add_child(title);
        titleRow.add_child(this._spacer());
        titleRow.add_child(value);

        const hint = new St.Label({
            text: 'Current balance',
            style_class: 'codexbar-credits-hint',
        });

        box.add_child(this._createDivider('codexbar-inline-divider'));
        box.add_child(titleRow);
        box.add_child(hint);
        box.visible = false;

        return {box, value, hint};
    }

    _buildFactsSection() {
        const box = new St.BoxLayout({
            vertical: true,
            style_class: 'codexbar-facts-section',
        });
        box.add_child(this._createDivider('codexbar-inline-divider'));

        const plan = this._createFactRow('Plan');
        const source = this._createFactRow('Source');
        const status = this._createFactRow('Status');
        const updated = this._createFactRow('Updated');

        box.add_child(plan.item);
        box.add_child(source.item);
        box.add_child(status.item);
        box.add_child(updated.item);

        return {box, plan, source, status, updated};
    }

    _buildActionItem() {
        const item = new PopupMenu.PopupBaseMenuItem({
            reactive: false,
            can_focus: false,
        });
        item.add_style_class_name('codexbar-actions-item');

        const actions = new St.BoxLayout({
            vertical: true,
            style_class: 'codexbar-actions-box',
            x_expand: true,
        });

        const switcher = new St.BoxLayout({
            style_class: 'codexbar-switcher-row',
            x_expand: true,
        });
        this._codexButton = this._createActionButton('Codex', () => {
            this._settings.set_string('default-provider', 'codex');
        });
        this._claudeButton = this._createActionButton('Claude', () => {
            this._settings.set_string('default-provider', 'claude');
        });
        switcher.add_child(this._codexButton);
        switcher.add_child(this._claudeButton);

        const tools = new St.BoxLayout({
            style_class: 'codexbar-tools-row',
            x_expand: true,
        });
        this._refreshButton = this._createActionButton('Refresh', () => this.refresh(true), 'utility');
        this._prefsButton = this._createActionButton('Prefs', () => {
            this._spawnDetached(['gnome-extensions', 'prefs', this._extension.uuid]);
        }, 'utility');
        this._configButton = this._createActionButton('Config', () => {
            const path = this._lastPayload?.config_path ?? DEFAULT_CONFIG_DIR;
            const target = path.endsWith('.json') ? GLib.path_get_dirname(path) : path;
            this._spawnDetached(['xdg-open', target]);
        }, 'utility');
        tools.add_child(this._refreshButton);
        tools.add_child(this._prefsButton);
        tools.add_child(this._configButton);

        actions.add_child(switcher);
        actions.add_child(tools);
        item.add_child(actions);
        return item;
    }

    _createMetricRow(titleText) {
        const item = new St.BoxLayout({
            vertical: true,
            style_class: 'codexbar-metric-row',
        });

        const topRow = new St.BoxLayout({
            style_class: 'codexbar-metric-top',
        });
        const title = new St.Label({
            text: titleText,
            style_class: 'codexbar-metric-title',
        });
        const percent = new St.Label({
            text: '--',
            style_class: 'codexbar-metric-percent',
        });
        topRow.add_child(title);
        topRow.add_child(this._spacer());
        topRow.add_child(percent);

        const meter = this._createMeter('codexbar-progress-track');
        const detail = new St.Label({
            text: 'Waiting for data…',
            style_class: 'codexbar-metric-detail',
        });

        item.add_child(topRow);
        item.add_child(meter.track);
        item.add_child(detail);

        return {item, percent, detail, meter};
    }

    _createFactRow(labelText) {
        const item = new St.BoxLayout({
            style_class: 'codexbar-fact-row',
        });
        const label = new St.Label({
            text: labelText,
            style_class: 'codexbar-fact-label',
        });
        const value = new St.Label({
            text: '--',
            style_class: 'codexbar-fact-value',
            x_align: Clutter.ActorAlign.END,
        });
        item.add_child(label);
        item.add_child(this._spacer());
        item.add_child(value);
        return {item, value};
    }

    _createActionButton(text, handler, variant = 'switcher') {
        const button = new St.Button({
            label: text,
            style_class: `codexbar-action-button codexbar-action-button-${variant}`,
            x_expand: true,
            can_focus: true,
            reactive: true,
            track_hover: true,
        });
        button.connect('clicked', handler);
        return button;
    }

    _createMeter(trackClass) {
        const track = new St.Bin({
            style_class: trackClass,
            x_expand: true,
        });
        const fill = new St.Widget({
            style_class: 'codexbar-progress-fill',
            x_expand: false,
            y_expand: true,
        });
        track.set_child(fill);
        return {track, fill};
    }

    _createDivider(extraClass = '') {
        const divider = new St.Widget({
            style_class: `codexbar-divider ${extraClass}`.trim(),
            x_expand: true,
        });
        return divider;
    }

    _spacer() {
        return new St.Widget({x_expand: true});
    }

    _connectSettings() {
        const reloadKeys = [
            'backend-path',
            'default-provider',
            'refresh-interval',
            'show-percentage',
            'keep-last-good-value',
        ];

        for (const key of reloadKeys) {
            const id = this._settings.connect(`changed::${key}`, () => {
                if (key === 'refresh-interval')
                    this._scheduleRefresh();

                this._render();
                if (key === 'backend-path')
                    this.refresh(true);
            });
            this._settingsSignals.push(id);
        }
    }

    _disconnectSettings() {
        for (const id of this._settingsSignals)
            this._settings.disconnect(id);
        this._settingsSignals = [];
    }

    _scheduleRefresh() {
        if (this._timeoutId) {
            GLib.Source.remove(this._timeoutId);
            this._timeoutId = 0;
        }

        const interval = Math.max(15, this._settings.get_uint('refresh-interval'));
        this._timeoutId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, interval, () => {
            this.refresh();
            return GLib.SOURCE_CONTINUE;
        });
    }

    refresh(force = false) {
        if (this._refreshInFlight && !force)
            return;

        const backendPath = this._resolveBackendPath();
        if (!GLib.file_test(backendPath, GLib.FileTest.EXISTS | GLib.FileTest.IS_EXECUTABLE)) {
            this._handleRefreshError(`Backend not found or not executable: ${backendPath}`);
            return;
        }

        this._refreshInFlight = true;
        const process = Gio.Subprocess.new(
            [backendPath, 'poll', '--provider', 'all'],
            Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_PIPE
        );

        process.communicate_utf8_async(null, null, (_process, result) => {
            try {
                const [, stdout, stderr] = process.communicate_utf8_finish(result);
                this._refreshInFlight = false;

                if (!process.get_successful()) {
                    const message = stderr?.trim() || 'Backend exited with a non-zero status.';
                    this._handleRefreshError(message);
                    return;
                }

                this._lastPayload = JSON.parse(stdout);
                this._lastError = null;
                this._render();
            } catch (error) {
                this._refreshInFlight = false;
                this._handleRefreshError(error instanceof Error ? error.message : `${error}`);
            }
        });
    }

    _handleRefreshError(message) {
        this._lastError = message;
        if (!this._settings.get_boolean('keep-last-good-value'))
            this._lastPayload = null;
        this._render();
    }

    _resolveBackendPath() {
        const configured = this._settings.get_string('backend-path').trim();
        return configured.length > 0 ? configured : DEFAULT_BACKEND_PATH;
    }

    _selectedProvider() {
        const selected = this._settings.get_string('default-provider').trim().toLowerCase();
        return selected === 'claude' ? 'claude' : 'codex';
    }

    _currentSnapshot() {
        const snapshots = this._lastPayload?.snapshots ?? [];
        if (snapshots.length === 0)
            return null;

        const selected = this._selectedProvider();
        return snapshots.find(snapshot => snapshot.provider === selected) ?? snapshots[0];
    }

    _render() {
        const snapshot = this._currentSnapshot();
        const effectiveProvider = snapshot?.provider ?? this._selectedProvider();
        const meta = PROVIDER_META[effectiveProvider] ?? PROVIDER_META.codex;
        const statusMeta = this._statusMeta(snapshot);
        const primary = snapshot?.usage?.primary ?? null;
        const secondary = snapshot?.usage?.secondary ?? null;
        const tertiary = snapshot?.usage?.tertiary ?? null;
        const error = this._resolvedError(snapshot);

        this._applyProviderTheme(meta, Boolean(error));

        this._header.chip.text = meta.badge;
        this._header.providerName.text = meta.label;
        this._header.heroValue.text = primary ? `${Math.round(primary.remaining_percent)}% left` : 'Offline';
        this._header.identity.text = this._identityText(snapshot);
        this._header.subtitle.text = this._subtitleText(snapshot, error);

        this._syncMeter(this._panelSessionMeter, primary?.remaining_percent, PANEL_METER_WIDTH, meta.accent, Boolean(error));
        this._syncMeter(this._panelWeeklyMeter, secondary?.remaining_percent, PANEL_METER_WIDTH, meta.accent, Boolean(error), true);

        this._syncMetricRow(this._usageSection.session, primary, meta.accent, 'Session');
        this._syncMetricRow(this._usageSection.weekly, secondary, meta.accent, 'Weekly');
        this._syncMetricRow(this._usageSection.tertiary, tertiary, meta.accent, 'Model cap');
        this._usageSection.box.visible = Boolean(primary || secondary || tertiary);

        const hasCredits = snapshot?.credits?.remaining === 0 || Boolean(snapshot?.credits?.remaining);
        this._creditsSection.box.visible = Boolean(hasCredits);
        if (hasCredits) {
            this._creditsSection.value.text = `${snapshot.credits.remaining.toFixed(2)} remaining`;
            this._creditsSection.hint.text = snapshot?.source === 'oauth'
                ? 'Current Codex balance from OAuth usage'
                : 'Current credit balance';
        }

        this._factsSection.plan.value.text = this._displayLoginMethod(snapshot?.identity?.login_method) ?? 'Unavailable';
        this._factsSection.source.value.text = snapshot?.source?.toUpperCase() ?? 'Unavailable';
        this._factsSection.status.value.text = statusMeta.label;
        this._factsSection.status.value.set_style(`color: ${statusMeta.color};`);
        this._factsSection.updated.value.text = this._updatedSummary(snapshot);

        this._errorBanner.box.visible = Boolean(error);
        this._errorBanner.body.text = error ?? '';

        this._panelBox.set_style(Boolean(error) ? 'opacity: 0.78;' : 'opacity: 1;');

        this._setButtonActive(this._codexButton, effectiveProvider === 'codex');
        this._setButtonActive(this._claudeButton, effectiveProvider === 'claude');
    }

    _syncMetricRow(metric, window, accent, titleText) {
        const visible = Boolean(window);
        metric.item.visible = visible;
        if (!visible)
            return;

        const remaining = Math.round(window.remaining_percent);
        metric.percent.text = `${remaining}% left`;
        metric.detail.text = this._windowDetail(window);
        this._syncMeter(metric.meter, window.remaining_percent, MENU_METER_WIDTH, accent, false);
    }

    _syncMeter(meter, remainingPercent, width, accent, isError, thin = false) {
        const trackColor = isError ? 'rgba(255, 117, 117, 0.14)' : 'rgba(255, 255, 255, 0.08)';
        meter.track.set_style(`background-color: ${trackColor};`);

        const normalized = typeof remainingPercent === 'number'
            ? Math.max(0, Math.min(100, remainingPercent))
            : 0;
        const fillWidth = normalized > 0 ? Math.max(2, Math.round(width * normalized / 100)) : 0;
        meter.fill.set_width(fillWidth);
        meter.fill.set_style(`background-color: ${accent};`);

        if (thin)
            meter.track.add_style_class_name('codexbar-progress-track-thin');
        else
            meter.track.remove_style_class_name('codexbar-progress-track-thin');
    }

    _applyProviderTheme(meta, isError) {
        this._card.set_style(`
            background: linear-gradient(180deg, rgba(20, 26, 36, 0.97), rgba(14, 18, 26, 0.97));
            border: 1px solid ${isError ? 'rgba(255, 117, 117, 0.28)' : meta.accentBorder};
            box-shadow: 0 16px 40px rgba(0, 0, 0, 0.36), inset 0 1px 0 rgba(255, 255, 255, 0.04);
        `);
        this._header.chip.set_style(`
            background-color: ${isError ? 'rgba(255, 117, 117, 0.16)' : meta.accentSoft};
            color: ${isError ? '#ff9393' : meta.accent};
        `);
        this._header.heroValue.set_style(`color: ${isError ? '#ff9393' : meta.accent};`);
        this._creditsSection.value.set_style(`color: ${meta.accent};`);
    }

    _setButtonActive(button, active) {
        if (active)
            button.add_style_class_name('codexbar-action-button-active');
        else
            button.remove_style_class_name('codexbar-action-button-active');
    }

    _statusMeta(snapshot) {
        const indicator = snapshot?.status?.indicator ?? 'unknown';
        return {
            indicator,
            ...(STATUS_META[indicator] ?? STATUS_META.unknown),
        };
    }

    _resolvedError(snapshot) {
        return this._lastError || snapshot?.error || null;
    }

    _identityText(snapshot) {
        const email = snapshot?.identity?.account_email?.trim();
        if (email)
            return email;

        const source = snapshot?.source?.toUpperCase();
        if (source)
            return `${source} source connected`;

        return 'Waiting for provider data…';
    }

    _subtitleText(snapshot, error) {
        if (error)
            return 'Showing degraded state until refresh succeeds';

        const parts = [];
        const loginMethod = this._displayLoginMethod(snapshot?.identity?.login_method);
        if (loginMethod)
            parts.push(loginMethod);
        const statusMeta = this._statusMeta(snapshot);
        if (statusMeta.indicator !== 'unknown')
            parts.push(statusMeta.label);
        if (parts.length === 0)
            parts.push('Backend connected');
        return parts.join(' · ');
    }

    _displayLoginMethod(rawValue) {
        if (!rawValue)
            return null;

        return rawValue
            .split(/[\s_-]+/)
            .filter(Boolean)
            .map(part => part.charAt(0).toUpperCase() + part.slice(1))
            .join(' ');
    }

    _windowDetail(window) {
        const parts = [];
        const resetText = this._formatReset(window);
        if (resetText)
            parts.push(`Resets ${resetText}`);
        if (window?.window_minutes)
            parts.push(this._windowDuration(window.window_minutes));
        return parts.join(' · ') || 'Usage window';
    }

    _windowDuration(windowMinutes) {
        if (windowMinutes >= 10080)
            return '7d window';
        if (windowMinutes >= 1440)
            return `${Math.round(windowMinutes / 1440)}d window`;
        if (windowMinutes >= 60)
            return `${Math.round(windowMinutes / 60)}h window`;
        return `${windowMinutes}m window`;
    }

    _updatedSummary(snapshot) {
        const updatedAt = snapshot?.usage?.updated_at_epoch_seconds ?? this._lastPayload?.updated_at_epoch_seconds;
        if (!updatedAt)
            return 'Unavailable';

        const delta = Math.max(0, Math.floor(GLib.DateTime.new_now_local().to_unix() - updatedAt));
        if (delta < 60)
            return `${delta}s ago`;
        if (delta < 3600)
            return `${Math.floor(delta / 60)}m ago`;
        return `${Math.floor(delta / 3600)}h ago`;
    }

    _formatReset(window) {
        if (window?.reset_description)
            return window.reset_description;

        let dateTime = null;
        if (window?.reset_at_epoch_seconds) {
            dateTime = GLib.DateTime.new_from_unix_local(window.reset_at_epoch_seconds);
        } else if (window?.reset_at_iso8601) {
            dateTime = GLib.DateTime.new_from_iso8601(window.reset_at_iso8601, null);
        }

        return dateTime ? dateTime.format('%b %e %H:%M') : null;
    }

    _spawnDetached(argv) {
        try {
            Gio.Subprocess.new(argv, Gio.SubprocessFlags.NONE);
        } catch (error) {
            this._handleRefreshError(error instanceof Error ? error.message : `${error}`);
        }
    }
});

export default class CodexBarGnomeExtension extends Extension {
    enable() {
        this._indicator = new CodexBarGnomeIndicator(this);
        Main.panel.addToStatusArea(this.uuid, this._indicator);
    }

    disable() {
        this._indicator?.destroy();
        this._indicator = null;
    }
}
