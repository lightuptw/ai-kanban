import React, { useEffect, useState } from "react";
import { useSelector, useDispatch } from "react-redux";
import { Snackbar, Alert } from "@mui/material";
import type { AlertColor } from "@mui/material";

import type { RootState, AppDispatch } from "../redux/store";
import type { NotificationType } from "../types/kanban";
import { clearLastAdded } from "../store/slices/notificationSlice";
import {
  sendBrowserNotification,
  playNotificationSound,
} from "../utils/browserNotifications";

const SEVERITY_MAP: Record<NotificationType, AlertColor> = {
  card_stage_changed: "info",
  ai_completed: "success",
  ai_question_pending: "warning",
  review_requested: "info",
  ai_error: "error",
};

const HIGH_PRIORITY_TYPES: NotificationType[] = [
  "ai_error",
  "ai_question_pending",
  "review_requested",
];

function NotificationSnackbar() {
  const dispatch = useDispatch<AppDispatch>();
  const lastAdded = useSelector(
    (state: RootState) => state.notifications.lastAdded
  );
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (lastAdded) {
      setOpen(true);

      if (HIGH_PRIORITY_TYPES.includes(lastAdded.notification_type)) {
        sendBrowserNotification(lastAdded.title, lastAdded.message);
        playNotificationSound();
      }
    }
  }, [lastAdded]);

  const handleClose = (
    _event?: React.SyntheticEvent | Event,
    reason?: string
  ) => {
    if (reason === "clickaway") return;
    setOpen(false);
    dispatch(clearLastAdded());
  };

  if (!lastAdded) return null;

  const severity = SEVERITY_MAP[lastAdded.notification_type] || "info";

  return (
    <Snackbar
      open={open}
      autoHideDuration={5000}
      onClose={handleClose}
      anchorOrigin={{ vertical: "bottom", horizontal: "right" }}
    >
      <Alert onClose={handleClose} severity={severity} variant="filled">
        <strong>{lastAdded.title}</strong>
        <br />
        {lastAdded.message}
      </Alert>
    </Snackbar>
  );
}

export default NotificationSnackbar;
