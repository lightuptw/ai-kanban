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
  AgentLog,
  AgentActivityResponse,
  CardVersion,
  BoardSettings,
  UpdateBoardSettingsRequest,
  DiffResult,
  MergeResult,
} from "../types/kanban";

interface AiQuestion {
  id: string;
  card_id: string;
  session_id: string;
  question: string;
  question_type: string;
  options: string;
  multiple: boolean;
  answer: string | null;
  answered_at: string | null;
  created_at: string;
}

const API_BASE_URL =
  import.meta.env.VITE_API_URL ||
  `${window.location.protocol}//${window.location.hostname}:21547`;

function buildHeaders(options?: RequestInit, token?: string | null): Headers {
  const headers = new Headers(options?.headers);
  headers.set("Content-Type", "application/json");

  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
  }

  return headers;
}

async function fetchAPI<T>(endpoint: string, options?: RequestInit, isRetry = false): Promise<T> {
  const token = localStorage.getItem("token");

  const response = await fetch(`${API_BASE_URL}${endpoint}`, {
    ...options,
    headers: buildHeaders(options, token),
  });

  if (response.status === 401 && !isRetry && token) {
    try {
      const { authService } = await import("./auth");
      const newToken = await authService.refresh();

      if (newToken) {
        return fetchAPI<T>(endpoint, {
          ...options,
          headers: buildHeaders(options, newToken),
        }, true);
      }
    } catch {
      const { authService } = await import("./auth");
      authService.logout();
      throw new Error("Unauthorized");
    }
  }

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

  generatePlan: (cardId: string) =>
    fetchAPI<Card>(`/api/cards/${cardId}/generate-plan`, { method: "POST" }),

  stopAi: (cardId: string) =>
    fetchAPI<Card>(`/api/cards/${cardId}/stop-ai`, { method: "POST" }),

  resumeAi: (cardId: string) =>
    fetchAPI<Card>(`/api/cards/${cardId}/resume-ai`, { method: "POST" }),

  getCardQuestions: (cardId: string) =>
    fetchAPI<AiQuestion[]>(`/api/cards/${cardId}/questions`),

  answerQuestion: (cardId: string, questionId: string, answer: string | string[]) =>
    fetchAPI<AiQuestion>(`/api/cards/${cardId}/questions/${questionId}/answer`, {
      method: "POST",
      body: JSON.stringify({ answer }),
    }),

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

  getCardLogs: (cardId: string) => fetchAPI<AgentLog[]>(`/api/cards/${cardId}/logs`),

  getAgentActivity: (cardId: string) =>
    fetchAPI<AgentActivityResponse>(`/api/cards/${cardId}/agent-activity`),

  getCardVersions: (cardId: string) => fetchAPI<CardVersion[]>(`/api/cards/${cardId}/versions`),

  restoreCardVersion: (cardId: string, versionId: string) =>
    fetchAPI<Card>(`/api/cards/${cardId}/versions/${versionId}/restore`, {
      method: "POST",
    }),

  getCardDiff: (cardId: string) =>
    fetchAPI<DiffResult>(`/api/cards/${cardId}/diff`),

  mergeCard: (cardId: string) =>
    fetchAPI<MergeResult>(`/api/cards/${cardId}/merge`, { method: "POST" }),

  createCardPr: (cardId: string, title?: string, body?: string) =>
    fetchAPI<{ url: string }>(`/api/cards/${cardId}/create-pr`, {
      method: "POST",
      body: JSON.stringify({ title, body }),
    }),

  rejectCard: (cardId: string, feedback?: string) =>
    fetchAPI<Card>(`/api/cards/${cardId}/reject`, {
      method: "POST",
      body: JSON.stringify({ feedback }),
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

  getBoardSettings: (boardId: string) =>
    fetchAPI<BoardSettings>(`/api/boards/${boardId}/settings`),

  updateBoardSettings: (boardId: string, data: UpdateBoardSettingsRequest) =>
    fetchAPI<BoardSettings>(`/api/boards/${boardId}/settings`, {
      method: "PUT",
      body: JSON.stringify(data),
    }),

  autoDetectBoardSettings: (boardId: string, codebasePath: string) =>
    fetchAPI<{ status: string; session_id?: string }>(`/api/boards/${boardId}/settings/auto-detect`, {
      method: "POST",
      body: JSON.stringify({ codebase_path: codebasePath }),
    }),

  cloneRepo: (boardId: string, githubUrl: string, clonePath: string, pat?: string) =>
    fetchAPI<{ success: boolean; codebase_path?: string; error?: string }>(
      `/api/boards/${boardId}/settings/clone-repo`,
      {
        method: "POST",
        body: JSON.stringify({ github_url: githubUrl, clone_path: clonePath, pat }),
      }
    ),

  getAutoDetectStatus: (boardId: string) =>
    fetchAPI<{ status: string; session_id: string; started_at: string }>(
      `/api/boards/${boardId}/settings/auto-detect-status`
    ),

  getAutoDetectLogs: (boardId: string, sessionId: string) =>
    fetchAPI<any>(`/api/boards/${boardId}/settings/auto-detect-logs?session_id=${sessionId}`),

  pickDirectory: () =>
    fetchAPI<{ path: string | null; paths: string[] }>("/api/pick-directory", { method: "POST" }),

  pickFiles: () =>
    fetchAPI<{ path: string | null; paths: string[] }>("/api/pick-files", { method: "POST" }),

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
