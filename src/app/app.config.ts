import { ApplicationConfig, provideBrowserGlobalErrorListeners } from '@angular/core';
import { AboutApi, TauriAboutApi } from './core/about/about-api';
import { EditorApi, TauriEditorApi } from './core/editor/editor-api';
import { RecordingApi, TauriRecordingApi } from './core/recording/recording-api';
import { McpApi, TauriMcpApi } from './core/mcp/mcp-api';
import { SettingsApi, TauriSettingsApi } from './core/settings/settings-api';
import { TauriWorkspaceApi, WorkspaceApi } from './core/workspace/workspace-api';

/**
 * Zoneless (Angular 22 default, no zone.js). Every backend facade is bound here; tests
 * replace them with the fakes from `src/testing/`.
 */
export const appConfig: ApplicationConfig = {
  providers: [
    provideBrowserGlobalErrorListeners(),
    { provide: AboutApi, useClass: TauriAboutApi },
    { provide: EditorApi, useClass: TauriEditorApi },
    { provide: RecordingApi, useClass: TauriRecordingApi },
    { provide: McpApi, useClass: TauriMcpApi },
    { provide: SettingsApi, useClass: TauriSettingsApi },
    { provide: WorkspaceApi, useClass: TauriWorkspaceApi },
  ],
};
