import React from "react";

import async from "./components/Async";
import { AuthGuard } from "./components/AuthGuard";
import DashboardLayout from "./layouts/Dashboard";

const KanbanBoard = async(() => import("./pages/kanban/KanbanBoard"));
const LoginPage = async(() => import("./pages/auth/LoginPage"));
const RegisterPage = async(() => import("./pages/auth/RegisterPage"));

const routes = [
  {
    path: "/login",
    element: <LoginPage />,
  },
  {
    path: "/register",
    element: <RegisterPage />,
  },
  {
    path: "/",
    element: (
      <AuthGuard>
        <DashboardLayout />
      </AuthGuard>
    ),
    children: [
      {
        path: "",
        element: <KanbanBoard />,
      },
    ],
  },
  {
    path: "*",
    element: (
      <AuthGuard>
        <DashboardLayout />
      </AuthGuard>
    ),
    children: [
      {
        path: "*",
        element: <KanbanBoard />,
      },
    ],
  },
];

export default routes;
