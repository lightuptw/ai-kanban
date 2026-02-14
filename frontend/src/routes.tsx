import React from "react";

import async from "./components/Async";
import DashboardLayout from "./layouts/Dashboard";

const KanbanBoard = async(() => import("./pages/kanban/KanbanBoard"));

const routes = [
  {
    path: "/",
    element: <DashboardLayout />,
    children: [
      {
        path: "",
        element: <KanbanBoard />,
      },
    ],
  },
  {
    path: "*",
    element: <DashboardLayout />,
    children: [
      {
        path: "*",
        element: <KanbanBoard />,
      },
    ],
  },
];

export default routes;
