import { API_BASE_URL } from "../constants";

export function avatarUrl(userId: string): string {
  return `${API_BASE_URL}/api/auth/avatar/${userId}`;
}

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

async function postAuth<T>(endpoint: string, payload?: unknown): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${endpoint}`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    credentials: "include",
    body: payload ? JSON.stringify(payload) : undefined,
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(error.error || `HTTP ${response.status}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

export function setUser(user: AuthUser) {
  sessionStorage.setItem(AUTH_USER_KEY, JSON.stringify(user));
}

export function getUser(): AuthUser | null {
  const userJson = sessionStorage.getItem(AUTH_USER_KEY);
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
  return Boolean(getUser());
}

export async function login(username: string, password: string) {
  const data = await postAuth<AuthResponse>("/api/auth/login", { username, password });
  setUser(data.user);
  return data;
}

export async function register(fields: RegisterFields) {
  const data = await postAuth<AuthResponse>("/api/auth/register", fields);
  setUser(data.user);
  return data;
}

export async function refresh() {
  const data = await postAuth<AuthResponse>("/api/auth/refresh");
  if (data.user) {
    setUser(data.user);
  }
}

export async function logout() {
  try {
    await postAuth<void>("/api/auth/logout");
  } catch {
    // Server logout failed — clear local state anyway
  }
  sessionStorage.removeItem(AUTH_USER_KEY);
  window.dispatchEvent(new Event("auth:logout"));
  window.location.href = "/login";
}

export async function uploadAvatar(file: File): Promise<AuthUser> {
  const formData = new FormData();
  formData.append("avatar", file);

  const response = await fetch(`${API_BASE_URL}/api/auth/me/avatar`, {
    method: "POST",
    credentials: "include",
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
  const response = await fetch(`${API_BASE_URL}/api/auth/me/avatar`, {
    method: "DELETE",
    credentials: "include",
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
  getUser,
  isAuthenticated,
  setUser,
  avatarUrl,
  uploadAvatar,
  deleteAvatar,
};
