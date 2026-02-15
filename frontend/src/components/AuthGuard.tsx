import React from "react";
import { Navigate } from "react-router-dom";

export const AuthGuard: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  if (!localStorage.getItem("token")) return <Navigate to="/login" replace />;
  return <>{children}</>;
};
