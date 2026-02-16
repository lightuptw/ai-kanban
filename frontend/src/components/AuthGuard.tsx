import React from "react";
import { Navigate } from "react-router-dom";
import { authService } from "../services/auth";

export const AuthGuard: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  if (!authService.isAuthenticated()) return <Navigate to="/login" replace />;
  return <>{children}</>;
};
