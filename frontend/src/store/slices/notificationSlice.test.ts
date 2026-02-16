import { describe, expect, it } from "vitest";
import reducer, {
  addNotificationFromWS,
  clearNotifications,
  clearLastAdded,
  fetchNotifications,
  markRead,
  markAllRead,
  deleteNotification,
} from "./notificationSlice";
import type { Notification } from "../../types/kanban";

const createNotification = (
  overrides: Partial<Notification> = {}
): Notification => ({
  id: "notification-1",
  user_id: "user-1",
  notification_type: "ai_completed",
  title: "AI task finished",
  message: "The AI has completed work on card-1",
  card_id: "card-1",
  board_id: "board-1",
  is_read: false,
  created_at: "2026-02-17T00:00:00Z",
  ...overrides,
});

const createState = () => ({
  notifications: [] as Notification[],
  unreadCount: 0,
  loading: false,
  error: null as string | null,
  lastAdded: null as Notification | null,
});

describe("notificationSlice", () => {
  it("has expected initial state", () => {
    const state = reducer(undefined, { type: "unknown" });

    expect(state).toEqual({
      notifications: [],
      unreadCount: 0,
      loading: false,
      error: null,
      lastAdded: null,
    });
  });

  it("addNotificationFromWS adds to front, increments unread, and sets lastAdded", () => {
    const existing = createNotification({ id: "notification-2" });
    const incoming = createNotification({ id: "notification-3", is_read: false });
    const initialState = {
      ...createState(),
      notifications: [existing],
      unreadCount: 1,
    };

    const state = reducer(initialState, addNotificationFromWS(incoming));

    expect(state.notifications).toHaveLength(2);
    expect(state.notifications[0].id).toBe("notification-3");
    expect(state.unreadCount).toBe(2);
    expect(state.lastAdded?.id).toBe("notification-3");
  });

  it("addNotificationFromWS with is_read=true does not increment unreadCount", () => {
    const incoming = createNotification({ id: "notification-4", is_read: true });
    const initialState = {
      ...createState(),
      unreadCount: 2,
    };

    const state = reducer(initialState, addNotificationFromWS(incoming));

    expect(state.notifications).toHaveLength(1);
    expect(state.unreadCount).toBe(2);
    expect(state.lastAdded?.id).toBe("notification-4");
  });

  it("clearNotifications resets notifications and unreadCount", () => {
    const initialState = {
      ...createState(),
      notifications: [
        createNotification({ id: "notification-5", is_read: false }),
        createNotification({ id: "notification-6", is_read: true }),
      ],
      unreadCount: 1,
    };

    const state = reducer(initialState, clearNotifications());

    expect(state.notifications).toEqual([]);
    expect(state.unreadCount).toBe(0);
  });

  it("clearLastAdded sets lastAdded to null", () => {
    const initialState = {
      ...createState(),
      lastAdded: createNotification({ id: "notification-7" }),
    };

    const state = reducer(initialState, clearLastAdded());

    expect(state.lastAdded).toBeNull();
  });

  it("fetchNotifications.pending sets loading=true and clears error", () => {
    const initialState = {
      ...createState(),
      error: "old error",
    };

    const state = reducer(initialState, { type: fetchNotifications.pending.type });

    expect(state.loading).toBe(true);
    expect(state.error).toBeNull();
  });

  it("fetchNotifications.fulfilled sets notifications and computes unreadCount", () => {
    const payload = [
      createNotification({ id: "notification-8", is_read: false }),
      createNotification({ id: "notification-9", is_read: true }),
      createNotification({ id: "notification-10", is_read: false }),
    ];
    const initialState = {
      ...createState(),
      loading: true,
    };

    const state = reducer(initialState, {
      type: fetchNotifications.fulfilled.type,
      payload,
    });

    expect(state.loading).toBe(false);
    expect(state.notifications).toEqual(payload);
    expect(state.unreadCount).toBe(2);
  });

  it("fetchNotifications.rejected sets error and loading=false", () => {
    const initialState = {
      ...createState(),
      loading: true,
    };

    const state = reducer(initialState, {
      type: fetchNotifications.rejected.type,
      error: { message: "Boom" },
    });

    expect(state.loading).toBe(false);
    expect(state.error).toBe("Boom");
  });

  it("markRead.fulfilled marks target notification read and decrements unreadCount", () => {
    const initialState = {
      ...createState(),
      notifications: [
        createNotification({ id: "notification-11", is_read: false }),
        createNotification({ id: "notification-12", is_read: true }),
      ],
      unreadCount: 1,
    };

    const state = reducer(initialState, {
      type: markRead.fulfilled.type,
      payload: { id: "notification-11" },
    });

    expect(state.notifications[0].is_read).toBe(true);
    expect(state.unreadCount).toBe(0);
  });

  it("markAllRead.fulfilled marks all notifications read and resets unreadCount", () => {
    const initialState = {
      ...createState(),
      notifications: [
        createNotification({ id: "notification-13", is_read: false }),
        createNotification({ id: "notification-14", is_read: false }),
      ],
      unreadCount: 2,
    };

    const state = reducer(initialState, { type: markAllRead.fulfilled.type });

    expect(state.notifications.every((n) => n.is_read)).toBe(true);
    expect(state.unreadCount).toBe(0);
  });

  it("deleteNotification.fulfilled removes notification and adjusts unreadCount when unread", () => {
    const initialState = {
      ...createState(),
      notifications: [
        createNotification({ id: "notification-15", is_read: false }),
        createNotification({ id: "notification-16", is_read: true }),
      ],
      unreadCount: 1,
    };

    const state = reducer(initialState, {
      type: deleteNotification.fulfilled.type,
      payload: "notification-15",
    });

    expect(state.notifications).toHaveLength(1);
    expect(state.notifications[0].id).toBe("notification-16");
    expect(state.unreadCount).toBe(0);
  });
});
