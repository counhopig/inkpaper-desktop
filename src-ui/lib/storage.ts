// Tiny localStorage wrapper for Server URL and (optionally) Admin Token.
// Tokens are kept in plain text here for now; the migration plan calls
// for moving them to the system Keychain in a later iteration.

const KEY_BASE_URL = "inkpaper.server.baseUrl";
const KEY_ADMIN_TOKEN = "inkpaper.server.adminToken";
const KEY_SELECTED_DEVICE = "inkpaper.server.selectedDevice";

export function loadServerBaseUrl(): string {
  return localStorage.getItem(KEY_BASE_URL) ?? "";
}

export function saveServerBaseUrl(value: string): void {
  if (value) localStorage.setItem(KEY_BASE_URL, value);
  else localStorage.removeItem(KEY_BASE_URL);
}

export function loadAdminToken(): string {
  return localStorage.getItem(KEY_ADMIN_TOKEN) ?? "";
}

export function saveAdminToken(value: string): void {
  if (value) localStorage.setItem(KEY_ADMIN_TOKEN, value);
  else localStorage.removeItem(KEY_ADMIN_TOKEN);
}

export function loadSelectedDeviceId(): string | null {
  const raw = localStorage.getItem(KEY_SELECTED_DEVICE);
  if (!raw) return null;
  // Device ids are now UUID strings; a leftover numeric id from before
  // the UUID migration can never match a real device, so drop it.
  return raw.trim() ? raw : null;
}

export function saveSelectedDeviceId(id: string | null): void {
  if (id == null) localStorage.removeItem(KEY_SELECTED_DEVICE);
  else localStorage.setItem(KEY_SELECTED_DEVICE, id);
}
