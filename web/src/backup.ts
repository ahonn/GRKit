import { GRADES, MAX_EFFECTS, type Cover, type Grade, type ImportedEffect } from './catalog';
import type { InstallMode } from './components/CardPanel';
import { EFFECT_ID_MAX } from './generated/site';
import type { Language } from './i18n';
import { base64, INPUTS, unbase64, type Input } from './import/names';
import { storedImport, type StoredImport } from './import/store';

/**
 * The backup of this browser: the imported effects, which IndexedDB keeps, and the settings,
 * which localStorage keeps. A file moves them to another browser, since both stores belong to
 * one browser.
 */
export const BACKUP_SCHEMA = 'grkit-backup/1';

export interface BackupSettings {
  language?: Language;
  installMode?: InstallMode;
  /** The checked effect IDs, or null when the page still shows its default selection. */
  selection?: readonly string[] | null;
}

export interface BackupContents {
  settings: BackupSettings;
  imports: readonly StoredImport[];
}

/** The plain JSON form of an import: the two byte arrays become base64 text. */
interface BackupImport {
  id: string;
  name: string;
  icon: string;
  cover: Cover;
  monochrome: boolean;
  preview: string;
  savedAt: number;
  imported: {
    blob: string;
    table: string;
    kind: 'lut' | 'native';
    source: string;
    grade?: Grade;
    input?: Input;
    warnings?: readonly string[];
    version?: string;
  };
}

interface BackupFile {
  schema: string;
  savedAt: string;
  settings: BackupSettings;
  imports: readonly BackupImport[];
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

/** The JSON form of one import, with its bytes as base64. */
function encodeImport(effect: ImportedEffect, savedAt: number): BackupImport {
  const imported = effect.imported;
  return {
    id: effect.id,
    name: effect.name,
    icon: effect.icon,
    cover: effect.cover,
    monochrome: effect.monochrome,
    preview: effect.preview,
    savedAt,
    imported:
      imported.kind === 'native'
        ? {
            kind: 'native',
            source: imported.source,
            version: imported.version,
            blob: base64(imported.blob),
            table: base64(imported.table),
          }
        : {
            kind: 'lut',
            source: imported.source,
            grade: imported.grade,
            input: imported.input,
            warnings: imported.warnings,
            blob: base64(imported.blob),
            table: base64(imported.table),
          },
  };
}

/**
 * The backup of `imports` and `settings` as a JSON file. `savedAt` orders the imports in the
 * backup, as the page shows them.
 */
export function createBackup(
  imports: readonly ImportedEffect[],
  settings: BackupSettings,
  savedAt = Date.now(),
): Blob {
  const file: BackupFile = {
    schema: BACKUP_SCHEMA,
    savedAt: new Date(savedAt).toISOString(),
    settings,
    imports: imports.map((effect, index) => encodeImport(effect, savedAt + index)),
  };
  return new Blob([JSON.stringify(file)], { type: 'application/json' });
}

/** One import of a backup, or null when its bytes or fields are not an effect that the page uses. */
export function decodeImport(value: unknown): StoredImport | null {
  if (!isRecord(value)) return null;
  const imported = value.imported;
  if (!isRecord(imported)) return null;
  const blob = typeof imported.blob === 'string' ? unbase64(imported.blob) : null;
  const table = typeof imported.table === 'string' ? unbase64(imported.table) : null;
  if (!blob || !table) return null;
  const stored = storedImport({ ...value, imported: { ...imported, blob, table } });
  if (!stored || !Number.isFinite(stored.savedAt) || stored.id.length > EFFECT_ID_MAX) {
    return null;
  }
  if (stored.imported.kind === 'lut') {
    const { grade, input, warnings } = stored.imported;
    if (
      !GRADES.includes(grade) ||
      !INPUTS.includes(input) ||
      !Array.isArray(warnings) ||
      !warnings.every((warning) => typeof warning === 'string')
    ) {
      return null;
    }
  }
  return stored;
}

/** The settings of a backup, keeping only the values that this version of the page can use. */
function readSettings(value: unknown): BackupSettings {
  if (!isRecord(value)) return {};
  const settings: BackupSettings = {};
  if (isLanguage(value.language)) settings.language = value.language;
  if (value.installMode === 'card' || value.installMode === 'zip') {
    settings.installMode = value.installMode;
  }
  if (value.selection === null) {
    settings.selection = null;
  } else if (Array.isArray(value.selection)) {
    const ids = value.selection.filter(
      (id): id is string => typeof id === 'string' && id.length > 0 && id.length <= EFFECT_ID_MAX,
    );
    settings.selection = ids.slice(0, MAX_EFFECTS);
  }
  return settings;
}

/**
 * The backup in `text`, or null when the file is not a backup of this version. A record that the
 * page cannot use is left out, so an older or partly damaged file still restores what it can.
 */
export function readBackup(text: string): BackupContents | null {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    return null;
  }
  if (!isRecord(value) || value.schema !== BACKUP_SCHEMA) return null;
  const imports = Array.isArray(value.imports)
    ? value.imports.flatMap((item) => decodeImport(item) ?? [])
    : [];
  return { settings: readSettings(value.settings), imports };
}
