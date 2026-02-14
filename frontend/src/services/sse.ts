import { Dispatch } from "@reduxjs/toolkit";
import { updateCardFromSSE, removeCardFromSSE } from "../store/slices/kanbanSlice";
import type { Card } from "../types/kanban";

const SSE_URL = (import.meta.env.VITE_API_URL || "http://localhost:3000") + "/api/events";

export class SSEManager {
  private eventSource: EventSource | null = null;
  private dispatch: Dispatch;
  private reconnectAttempts = 0;
  private maxReconnectDelay = 30000;

  constructor(dispatch: Dispatch) {
    this.dispatch = dispatch;
  }

  connect() {
    if (this.eventSource) {
      this.eventSource.close();
    }

    console.log("[SSE] Connecting to", SSE_URL);
    this.eventSource = new EventSource(SSE_URL);

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

    switch (event.type || event.event) {
      case "CardCreated":
      case "CardUpdated":
      case "CardMoved":
        if (event.card) {
          this.dispatch(updateCardFromSSE(event.card as Card));
        }
        break;

      case "CardDeleted":
        if (event.card_id) {
          this.dispatch(removeCardFromSSE(event.card_id));
        }
        break;

      case "AiStatusChanged":
        break;

      default:
        console.log("[SSE] Unknown event type:", event.type || event.event);
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
