const API_BASE_URL =
  import.meta.env.VITE_API_URL ||
  `${window.location.protocol}//${window.location.hostname}:21547`;

const AUTH_USER_KEY = "auth_user";

export type AuthUser = {
  id: string;
  username: string;
  nickname: string;
  first_name: string;
  last_name: string;
  email: string;
  tenant_id: string;
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
  }
  sessionStorage.removeItem(AUTH_USER_KEY);
  window.dispatchEvent(new Event("auth:logout"));
  window.location.href = "/login";
}

export const authService = {
  login,
  register,
  refresh,
  logout,
  getUser,
  isAuthenticated,
  setUser,
};
