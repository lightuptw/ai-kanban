import {
  updateCardFromSSE,
  removeCardFromSSE,
  updateCardAiStatus,
  moveCardInStore,
  updateBoardFromSSE,
  removeBoardFromSSE,
  setAutoDetectStatus,
  updateCardSubtaskFromWS,
  removeCardSubtaskFromWS,
  updateCardCommentFromWS,
  removeCardCommentFromWS,
} from "../store/slices/kanbanSlice";
import { addNotificationFromWS } from "../store/slices/notificationSlice";
import type { AppDispatch, RootState } from "../redux/store";
import type { Card, Board, Notification } from "../types/kanban";
import { API_BASE_URL } from "../constants";

/** Shape of WebSocket event payloads from the backend. */
interface WsEventData {
  type: string;
  card?: Card;
  card_id?: string;
  board?: Board;
  board_id?: string;
  from_stage?: string;
  to_stage?: string;
  status?: string;
  progress?: string | Record<string, unknown>;
  stage?: string;
  ai_session_id?: string | null;
  ai_status?: string;
  subtask?: { id: string; completed: boolean };
  subtask_id?: string;
  comment?: { id: string; card_id: string; author: string; content: string; created_at: string };
  comment_id?: string;
  label_id?: string;
  session_id?: string;
  elapsed_seconds?: number;
  message?: string;
  question?: Record<string, unknown>;
  notification?: Notification;
}

const WS_DEBUG = import.meta.env.DEV;
const wsLog = (...args: unknown[]) => WS_DEBUG && console.log(...args);
const wsError = (...args: unknown[]) => WS_DEBUG && console.error(...args);

export class WebSocketManager {
  private ws: WebSocket | null = null;
  private dispatch: AppDispatch;
  private getState: () => RootState;
  private reconnectAttempts = 0;
  private maxReconnectDelay = 30000;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private isManualDisconnect = false;

  constructor(dispatch: AppDispatch, getState: () => RootState) {
    this.dispatch = dispatch;
    this.getState = getState;
  }

  connect() {
    this.isManualDisconnect = false;

    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    if (this.ws) {
      this.ws.close();
    }

    const token = localStorage.getItem("token") || "";
    if (!token) {
      wsLog("[WS] No auth token, skipping connection");
      return;
    }

    const apiUrl = new URL(API_BASE_URL);
    const protocol = apiUrl.protocol === "https:" ? "wss:" : "ws:";
    const wsUrl = `${protocol}//${apiUrl.host}/ws/events?token=${encodeURIComponent(token)}`;

    wsLog("[WS] Connecting to", wsUrl);
    this.ws = new WebSocket(wsUrl);

    this.ws.onopen = () => {
      wsLog("[WS] Connected");
      this.reconnectAttempts = 0;
    };

    this.ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        this.handleEvent(data);
      } catch (error) {
        wsError("[WS] Failed to parse event:", error);
      }
    };

    this.ws.onclose = () => {
      wsLog("[WS] Connection closed");
      if (!this.isManualDisconnect) {
        this.reconnect();
      }
    };

    this.ws.onerror = (error) => {
      wsError("[WS] Error:", error);
    };
  }

  private handleEvent(event: WsEventData) {
    wsLog("[WS] Event:", event.type);
    const eventType = event.type;

    switch (eventType) {
      case "cardCreated":
        if (event.card) {
          this.dispatch(updateCardFromSSE(event.card as Card));
        }
        break;

      case "cardUpdated":
        if (event.card) {
          this.dispatch(updateCardFromSSE(event.card as Card));
        }
        break;

      case "cardMoved":
        if (event.card_id && event.from_stage && event.to_stage) {
          this.dispatch(
            moveCardInStore({
              cardId: event.card_id,
              fromStage: event.from_stage,
              toStage: event.to_stage,
            })
          );
        }
        break;

      case "cardDeleted":
        if (event.card_id) {
          this.dispatch(removeCardFromSSE(event.card_id));
        }
        break;

      case "aiStatusChanged":
        if (event.card_id && event.status) {
          this.dispatch(
            updateCardAiStatus({
              cardId: event.card_id,
              status: event.status,
              progress: event.progress,
              stage: event.stage,
              ai_session_id: event.ai_session_id,
            })
          );
        }
        break;

      case "questionCreated":
      case "questionAnswered":
        if (event.card_id) {
          this.dispatch(
            updateCardAiStatus({
              cardId: event.card_id,
              status:
                event.ai_status ||
                (eventType === "questionCreated" ? "waiting_input" : "working"),
            })
          );
        }
        break;

      case "subtaskCreated":
      case "subtaskUpdated":
      case "subtaskToggled":
        if (event.card_id && event.subtask) {
          const subtaskEventMap: Record<string, "created" | "updated" | "toggled"> = {
            subtaskCreated: "created",
            subtaskUpdated: "updated",
            subtaskToggled: "toggled",
          };
          this.dispatch(
            updateCardSubtaskFromWS({
              cardId: event.card_id,
              subtask: event.subtask,
              eventType: subtaskEventMap[eventType],
            })
          );
        }
        break;

      case "subtaskDeleted":
        if (event.card_id && event.subtask_id) {
          this.dispatch(
            removeCardSubtaskFromWS({
              cardId: event.card_id,
              subtaskId: event.subtask_id,
            })
          );
        }
        break;

      case "commentCreated":
      case "commentUpdated":
        if (event.card_id && event.comment) {
          this.dispatch(
            updateCardCommentFromWS({
              cardId: event.card_id,
              comment: event.comment,
              eventType: eventType === "commentCreated" ? "created" : "updated",
            })
          );
        }
        break;

      case "commentDeleted":
        if (event.card_id && event.comment_id) {
          this.dispatch(
            removeCardCommentFromWS({
              cardId: event.card_id,
              commentId: event.comment_id,
            })
          );
        }
        break;

      case "boardCreated":
      case "boardUpdated":
        if (event.board) {
          this.dispatch(updateBoardFromSSE(event.board as Board));
        }
        break;

      case "boardDeleted":
        if (event.board_id) {
          this.dispatch(removeBoardFromSSE(event.board_id));
        }
        break;

      case "labelAdded":
      case "labelRemoved":
        break;

      case "autoDetectStatus":
        if (event.board_id && event.status) {
          this.dispatch(
            setAutoDetectStatus({
              boardId: event.board_id,
              status: event.status,
            })
          );
          window.dispatchEvent(new CustomEvent("autoDetectStatus", { detail: event }));
        }
        break;

      case "notificationCreated":
        if (event.notification) {
          this.dispatch(addNotificationFromWS(event.notification));
        }
        break;

      case "connected":
        wsLog("[WS] Server confirmed connection");
        break;

      default:
        wsLog("[WS] Unknown event:", eventType);
    }
  }

  private reconnect() {
    const delay = Math.min(
      1000 * Math.pow(2, this.reconnectAttempts),
      this.maxReconnectDelay
    );
    this.reconnectAttempts++;

    wsLog(`[WS] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);

    this.reconnectTimer = setTimeout(() => {
      this.connect();
    }, delay);
  }

  disconnect() {
    this.isManualDisconnect = true;

    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }
}
