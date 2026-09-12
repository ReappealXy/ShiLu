import { invoke } from "@tauri-apps/api/core";
import type { ModelConfig } from "./model";

export type ThemePreference = "light" | "dark";

export type AppSettings = {
  theme: ThemePreference;
  ocrModel: ModelConfig | null;
  polishModel: ModelConfig | null;
};

export async function getStoredTheme(): Promise<ThemePreference> {
  const settings = await invoke<AppSettings>("get_app_settings");
  return settings.theme;
}

export function setStoredTheme(theme: ThemePreference): Promise<AppSettings> {
  return invoke<AppSettings>("set_theme_preference", { theme });
}

export function applyTheme(theme: ThemePreference): void {
  document.documentElement.dataset.theme = theme;
}
