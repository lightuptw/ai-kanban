import { useState, useEffect, useCallback } from "react";
import type { AuthUser } from "../services/auth";
import { getUser, setUser as persistUser } from "../services/auth";
import { api } from "../services/api";

const AUTH_UPDATE_EVENT = "auth:user-updated";

export function useAuth() {
  const [user, setUserState] = useState<AuthUser | null>(() => getUser());

  useEffect(() => {
    const handleUpdate = () => {
      setUserState(getUser());
    };

    window.addEventListener(AUTH_UPDATE_EVENT, handleUpdate);
    window.addEventListener("storage", handleUpdate);

    return () => {
      window.removeEventListener(AUTH_UPDATE_EVENT, handleUpdate);
      window.removeEventListener("storage", handleUpdate);
    };
  }, []);

  const updateUser = useCallback((updated: AuthUser) => {
    persistUser(updated);
    setUserState(updated);
    window.dispatchEvent(new Event(AUTH_UPDATE_EVENT));
  }, []);

  const refreshUser = useCallback(async () => {
    try {
      const fresh = await api.getMe();
      persistUser(fresh);
      setUserState(fresh);
      window.dispatchEvent(new Event(AUTH_UPDATE_EVENT));
      return fresh;
    } catch {
      return null;
    }
  }, []);

  return { user, updateUser, refreshUser };
}
