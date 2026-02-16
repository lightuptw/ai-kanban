import {
  updateCardFromSSE,
  removeCardFromSSE,
  updateCardAiStatus,
  moveCardInStore,
  updateBoardFromSSE,
  removeBoardFromSSE,
  setAutoDetectStatus,
  fetchBoard,
  fetchBoards,
} from "../store/slices/kanbanSlice";
import type { AppDispatch, RootState } from "../redux/store";
import type { Card, Board } from "../types/kanban";

const SSE_URL =
  (import.meta.env.VITE_API_URL ||
    `${window.location.protocol}//${window.location.hostname}:21547`) +
  "/api/events";

export class SSEManager {
  private eventSource: EventSource | null = null;
  private dispatch: AppDispatch;
  private getState: () => RootState;
  private reconnectAttempts = 0;
  private maxReconnectDelay = 30000;

  constructor(dispatch: AppDispatch, getState: () => RootState) {
    this.dispatch = dispatch;
    this.getState = getState;
  }

  connect() {
    if (this.eventSource) {
      this.eventSource.close();
    }

    const token = localStorage.getItem("token") || "";
    if (!token) {
      console.log("[SSE] No auth token, skipping connection");
      return;
    }
    const sseUrl = `${SSE_URL}?token=${encodeURIComponent(token)}`;

    console.log("[SSE] Connecting to", sseUrl);
    this.eventSource = new EventSource(sseUrl);

    this.eventSource.onopen = () => {
      console.log("[SSE] Connected");
      this.reconnectAttempts = 0;
    };

    this.eventSource.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        this.handleEvent(data);
      } catch (error) {
        console.error("[SSE] Failed to parse event:", error);
      }
    };

    this.eventSource.onerror = () => {
      console.error("[SSE] Connection error");
      this.eventSource?.close();
      this.reconnect();
    };
  }

  private handleEvent(event: any) {
    console.log("[SSE] Event received:", event);
    const eventType = event.type || event.event;

    switch (eventType) {
      case "cardCreated":
      case "CardCreated":
        if (event.card) {
          this.dispatch(updateCardFromSSE(event.card as Card));
        } else {
          const boardId = this.getState().kanban.activeBoardId;
          if (boardId) this.dispatch(fetchBoard(boardId));
        }
        break;

      case "cardUpdated":
      case "CardUpdated":
        if (event.card) {
          this.dispatch(updateCardFromSSE(event.card as Card));
        } else {
          const boardId = this.getState().kanban.activeBoardId;
          if (boardId) this.dispatch(fetchBoard(boardId));
        }
        break;

      case "cardMoved":
      case "CardMoved":
        if (event.card) {
          this.dispatch(updateCardFromSSE(event.card as Card));
        } else if (event.card_id && event.from_stage && event.to_stage) {
          this.dispatch(moveCardInStore({
            cardId: event.card_id,
            fromStage: event.from_stage,
            toStage: event.to_stage,
          }));
        }
        break;

      case "cardDeleted":
      case "CardDeleted":
        if (event.card_id) {
          this.dispatch(removeCardFromSSE(event.card_id));
        }
        break;

      case "aiStatusChanged":
      case "AiStatusChanged":
        if (event.card_id && event.status) {
          this.dispatch(updateCardAiStatus({
            cardId: event.card_id,
            status: event.status,
            progress: event.progress,
            stage: event.stage,
            ai_session_id: event.ai_session_id,
          }));
        }
        break;

      case "questionCreated":
      case "QuestionCreated":
      case "questionAnswered":
      case "QuestionAnswered":
        if (event.card_id) {
          this.dispatch(updateCardAiStatus({
            cardId: event.card_id,
            status: event.ai_status || (eventType === "QuestionCreated" || eventType === "questionCreated" ? "waiting_input" : "working"),
          }));
        }
        break;

      case "subtaskCreated":
      case "SubtaskCreated":
      case "subtaskUpdated":
      case "SubtaskUpdated":
      case "subtaskToggled":
      case "SubtaskToggled":
        {
          const boardId = this.getState().kanban.activeBoardId;
          if (boardId) this.dispatch(fetchBoard(boardId));
        }
        break;

      case "subtaskDeleted":
      case "SubtaskDeleted":
        {
          const boardId = this.getState().kanban.activeBoardId;
          if (boardId) this.dispatch(fetchBoard(boardId));
        }
        break;

      case "commentCreated":
      case "CommentCreated":
      case "commentUpdated":
      case "CommentUpdated":
      case "commentDeleted":
      case "CommentDeleted":
        {
          const boardId = this.getState().kanban.activeBoardId;
          if (boardId) this.dispatch(fetchBoard(boardId));
        }
        break;

      case "boardCreated":
      case "BoardCreated":
        if (event.board) {
          this.dispatch(updateBoardFromSSE(event.board as Board));
        } else {
          this.dispatch(fetchBoards());
        }
        break;

      case "boardUpdated":
      case "BoardUpdated":
        if (event.board) {
          this.dispatch(updateBoardFromSSE(event.board as Board));
        } else {
          this.dispatch(fetchBoards());
        }
        break;

      case "boardDeleted":
      case "BoardDeleted":
        if (event.board_id) {
          this.dispatch(removeBoardFromSSE(event.board_id));
        }
        break;

      case "labelAdded":
      case "LabelAdded":
      case "labelRemoved":
      case "LabelRemoved":
        {
          const boardId = this.getState().kanban.activeBoardId;
          if (boardId) this.dispatch(fetchBoard(boardId));
        }
        break;

      case "autoDetectStatus":
      case "AutoDetectStatus":
        if (event.board_id) {
          this.dispatch(
            setAutoDetectStatus({
              boardId: event.board_id,
              status: event.status,
            })
          );
          window.dispatchEvent(new CustomEvent("autoDetectStatus", { detail: event }));
        }
        break;

      default:
        console.log("[SSE] Unknown event type:", eventType);
    }
  }

  private reconnect() {
    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), this.maxReconnectDelay);
    this.reconnectAttempts++;

    console.log(`[SSE] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);

    setTimeout(() => {
      this.connect();
    }, delay);
  }

  disconnect() {
    if (this.eventSource) {
      this.eventSource.close();
      this.eventSource = null;
    }
  }
}
