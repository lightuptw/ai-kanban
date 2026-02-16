import { configureStore, ThunkAction, Action } from "@reduxjs/toolkit";
import kanbanReducer from "../store/slices/kanbanSlice";
import notificationReducer from "../store/slices/notificationSlice";

export const store = configureStore({
  reducer: {
    kanban: kanbanReducer,
    notifications: notificationReducer,
  },
});

export type AppDispatch = typeof store.dispatch;
export type RootState = ReturnType<typeof store.getState>;
export type AppThunk<ReturnType = void> = ThunkAction<
  ReturnType,
  RootState,
  unknown,
  Action<string>
>;
