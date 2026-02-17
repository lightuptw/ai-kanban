import { API_BASE_URL } from "../constants";

export function avatarUrl(userId: string): string {
  return `${API_BASE_URL}/api/auth/avatar/${userId}`;
}

const TOKEN_KEY = "token";
const REFRESH_TOKEN_KEY = "refresh_token";
const AUTH_USER_KEY = "auth_user";

export type AuthUser = {
  id: string;
  username: string;
  nickname: string;
  first_name: string;
  last_name: string;
  email: string;
  tenant_id: string;
  has_avatar: boolean;
  avatar_url: string | null;
  profile_completed: boolean;
};

export type AuthResponse = {
  token: string;
  refresh_token: string;
  user: AuthUser;
};

type RegisterFields = {
  username: string;
  nickname: string;
  password: string;
  first_name?: string;
  last_name?: string;
  email?: string;
};

async function postAuth<T>(endpoint: string, payload: unknown): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${endpoint}`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(error.error || `HTTP ${response.status}`);
  }

  return response.json();
}

export function setTokens(token: string, refreshToken: string) {
  localStorage.setItem(TOKEN_KEY, token);
  localStorage.setItem(REFRESH_TOKEN_KEY, refreshToken);
}

export function setUser(user: AuthUser) {
  localStorage.setItem(AUTH_USER_KEY, JSON.stringify(user));
}

export function getToken() {
  return localStorage.getItem(TOKEN_KEY);
}

export function getUser(): AuthUser | null {
  const userJson = localStorage.getItem(AUTH_USER_KEY);
  if (!userJson) {
    return null;
  }

  try {
    return JSON.parse(userJson) as AuthUser;
  } catch {
    return null;
  }
}

export function isAuthenticated() {
  return Boolean(getToken());
}

export async function login(username: string, password: string) {
  const data = await postAuth<AuthResponse>("/api/auth/login", { username, password });
  setTokens(data.token, data.refresh_token);
  setUser(data.user);
  return data;
}

export async function register(fields: RegisterFields) {
  const data = await postAuth<AuthResponse>("/api/auth/register", fields);
  setTokens(data.token, data.refresh_token);
  setUser(data.user);
  return data;
}

export async function refresh() {
  const refreshToken = localStorage.getItem(REFRESH_TOKEN_KEY);
  if (!refreshToken) {
    throw new Error("No refresh token");
  }

  const data = await postAuth<Partial<AuthResponse>>("/api/auth/refresh", {
    refresh_token: refreshToken,
  });

  if (!data.token) {
    throw new Error("Refresh failed");
  }

  setTokens(data.token, data.refresh_token || refreshToken);
  if (data.user) {
    setUser(data.user);
  }

  return data.token;
}

export function logout() {
  window.dispatchEvent(new Event("auth:logout"));
  localStorage.removeItem(TOKEN_KEY);
  localStorage.removeItem(REFRESH_TOKEN_KEY);
  localStorage.removeItem(AUTH_USER_KEY);
  window.location.href = "/login";
}

export async function uploadAvatar(file: File): Promise<AuthUser> {
  const token = getToken();
  if (!token) throw new Error("Not authenticated");

  const formData = new FormData();
  formData.append("avatar", file);

  const response = await fetch(`${API_BASE_URL}/api/auth/avatar`, {
    method: "POST",
    headers: { Authorization: `Bearer ${token}` },
    body: formData,
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(error.error || `HTTP ${response.status}`);
  }

  const user: AuthUser = await response.json();
  setUser(user);
  return user;
}

export async function deleteAvatar(): Promise<void> {
  const token = getToken();
  if (!token) throw new Error("Not authenticated");

  const response = await fetch(`${API_BASE_URL}/api/auth/avatar`, {
    method: "DELETE",
    headers: { Authorization: `Bearer ${token}` },
  });

  if (!response.ok && response.status !== 204) {
    throw new Error(`HTTP ${response.status}`);
  }

  const currentUser = getUser();
  if (currentUser) {
    setUser({ ...currentUser, has_avatar: false });
  }
}

export const authService = {
  login,
  register,
  refresh,
  logout,
  getToken,
  getUser,
  isAuthenticated,
  setTokens,
  setUser,
  avatarUrl,
  uploadAvatar,
  deleteAvatar,
};
