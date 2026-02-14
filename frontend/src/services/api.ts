import type {
  BoardResponse,
  Card,
  CreateCardRequest,
  UpdateCardRequest,
  MoveCardRequest,
  Subtask,
  CreateSubtaskRequest,
  UpdateSubtaskRequest,
  Label,
  Comment,
  CreateCommentRequest,
  Board,
  CreateBoardRequest,
  UpdateBoardRequest,
  ReorderBoardRequest,
} from "../types/kanban";

const API_BASE_URL = import.meta.env.VITE_API_URL || "http://localhost:3000";

async function fetchAPI<T>(endpoint: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${endpoint}`, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...options?.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(error.error || `HTTP ${response.status}`);
  }

  if (response.status === 204 || response.headers.get("content-length") === "0") {
    return undefined as T;
  }

  return response.json();
}

export const api = {
  getBoard: (boardId?: string) =>
    fetchAPI<BoardResponse>(boardId ? `/api/board?board_id=${boardId}` : "/api/board"),

  createCard: (data: CreateCardRequest) =>
    fetchAPI<Card>("/api/cards", {
      method: "POST",
      body: JSON.stringify(data),
    }),

  getCard: (id: string) => fetchAPI<Card>(`/api/cards/${id}`),

  updateCard: (id: string, data: UpdateCardRequest) =>
    fetchAPI<Card>(`/api/cards/${id}`, {
      method: "PATCH",
      body: JSON.stringify(data),
    }),

  moveCard: (id: string, data: MoveCardRequest) =>
    fetchAPI<Card>(`/api/cards/${id}/move`, {
      method: "PATCH",
      body: JSON.stringify(data),
    }),

  deleteCard: (id: string) =>
    fetchAPI<void>(`/api/cards/${id}`, {
      method: "DELETE",
    }),

  getSubtasks: (cardId: string) => fetchAPI<Subtask[]>(`/api/cards/${cardId}/subtasks`),

  createSubtask: (cardId: string, data: CreateSubtaskRequest) =>
    fetchAPI<Subtask>(`/api/cards/${cardId}/subtasks`, {
      method: "POST",
      body: JSON.stringify(data),
    }),

  updateSubtask: (id: string, data: UpdateSubtaskRequest) =>
    fetchAPI<Subtask>(`/api/subtasks/${id}`, {
      method: "PATCH",
      body: JSON.stringify(data),
    }),

  deleteSubtask: (id: string) =>
    fetchAPI<void>(`/api/subtasks/${id}`, {
      method: "DELETE",
    }),

  getLabels: () => fetchAPI<Label[]>("/api/labels"),

  addCardLabel: (cardId: string, labelId: string) =>
    fetchAPI<void>(`/api/cards/${cardId}/labels/${labelId}`, {
      method: "POST",
    }),

  removeCardLabel: (cardId: string, labelId: string) =>
    fetchAPI<void>(`/api/cards/${cardId}/labels/${labelId}`, {
      method: "DELETE",
    }),

  getComments: (cardId: string) => fetchAPI<Comment[]>(`/api/cards/${cardId}/comments`),

  createComment: (cardId: string, data: CreateCommentRequest) =>
    fetchAPI<Comment>(`/api/cards/${cardId}/comments`, {
      method: "POST",
      body: JSON.stringify(data),
    }),

  // Boards
  listBoards: () => fetchAPI<Board[]>("/api/boards"),

  createBoard: (data: CreateBoardRequest) =>
    fetchAPI<Board>("/api/boards", {
      method: "POST",
      body: JSON.stringify(data),
    }),

  updateBoard: (id: string, data: UpdateBoardRequest) =>
    fetchAPI<Board>(`/api/boards/${id}`, {
      method: "PATCH",
      body: JSON.stringify(data),
    }),

  deleteBoard: (id: string) =>
    fetchAPI<void>(`/api/boards/${id}`, {
      method: "DELETE",
    }),

  reorderBoard: (id: string, data: ReorderBoardRequest) =>
    fetchAPI<Board>(`/api/boards/${id}/reorder`, {
      method: "PATCH",
      body: JSON.stringify(data),
    }),

  // Settings
  getSetting: (key: string) =>
    fetchAPI<{ key: string; value: string; updated_at: string }>(
      `/api/settings/${key}`
    ),

  setSetting: (key: string, value: string) =>
    fetchAPI<{ key: string; value: string; updated_at: string }>(
      `/api/settings/${key}`,
      {
        method: "PUT",
        body: JSON.stringify({ value }),
      }
    ),
};
