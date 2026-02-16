import { createSlice, createAsyncThunk, PayloadAction } from "@reduxjs/toolkit";
import type { Notification } from "../../types/kanban";
import { api } from "../../services/api";

interface NotificationState {
  notifications: Notification[];
  unreadCount: number;
  loading: boolean;
  error: string | null;
  lastAdded: Notification | null;
}

const initialState: NotificationState = {
  notifications: [],
  unreadCount: 0,
  loading: false,
  error: null,
  lastAdded: null,
};

export const fetchNotifications = createAsyncThunk(
  "notifications/fetch",
  async (unreadOnly?: boolean) => {
    return await api.getNotifications(unreadOnly);
  }
);

export const markRead = createAsyncThunk(
  "notifications/markRead",
  async (id: string) => {
    return await api.markNotificationRead(id);
  }
);

export const markAllRead = createAsyncThunk(
  "notifications/markAllRead",
  async () => {
    await api.markAllNotificationsRead();
  }
);

export const deleteNotification = createAsyncThunk(
  "notifications/delete",
  async (id: string) => {
    await api.deleteNotification(id);
    return id;
  }
);

const notificationSlice = createSlice({
  name: "notifications",
  initialState,
  reducers: {
    addNotificationFromWS(state, action: PayloadAction<Notification>) {
      state.notifications.unshift(action.payload);
      if (!action.payload.is_read) {
        state.unreadCount += 1;
      }
      state.lastAdded = action.payload;
    },
    clearNotifications(state) {
      state.notifications = [];
      state.unreadCount = 0;
    },
    clearLastAdded(state) {
      state.lastAdded = null;
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(fetchNotifications.pending, (state) => {
        state.loading = true;
        state.error = null;
      })
      .addCase(fetchNotifications.fulfilled, (state, action) => {
        state.loading = false;
        state.notifications = action.payload;
        state.unreadCount = action.payload.filter((n) => !n.is_read).length;
      })
      .addCase(fetchNotifications.rejected, (state, action) => {
        state.loading = false;
        state.error = action.error.message || "Failed to fetch notifications";
      })
      .addCase(markRead.fulfilled, (state, action) => {
        const notification = state.notifications.find(
          (n) => n.id === action.payload.id
        );
        if (notification && !notification.is_read) {
          notification.is_read = true;
          state.unreadCount = Math.max(0, state.unreadCount - 1);
        }
      })
      .addCase(markAllRead.fulfilled, (state) => {
        state.notifications.forEach((n) => {
          n.is_read = true;
        });
        state.unreadCount = 0;
      })
      .addCase(deleteNotification.fulfilled, (state, action) => {
        const idx = state.notifications.findIndex(
          (n) => n.id === action.payload
        );
        if (idx !== -1) {
          if (!state.notifications[idx].is_read) {
            state.unreadCount = Math.max(0, state.unreadCount - 1);
          }
          state.notifications.splice(idx, 1);
        }
      });
  },
});

export const { addNotificationFromWS, clearNotifications, clearLastAdded } =
  notificationSlice.actions;
export default notificationSlice.reducer;
