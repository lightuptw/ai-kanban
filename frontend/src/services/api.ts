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
  ConflictDetail,
  FileResolution,
  MergeResult,
  Notification,
} from "../types/kanban";
import { API_BASE_URL } from "../constants";

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

function buildHeaders(options?: RequestInit): Headers {
  const headers = new Headers(options?.headers);
  headers.set("Content-Type", "application/json");
  return headers;
}

async function fetchAPI<T>(endpoint: string, options?: RequestInit, isRetry = false): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${endpoint}`, {
    ...options,
    credentials: "include",
    headers: buildHeaders(options),
  });

  if (response.status === 401 && !isRetry) {
    try {
      const { authService } = await import("./auth");
      await authService.refresh();
      return fetchAPI<T>(endpoint, options, true);
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

async function fetchMultipart<T>(endpoint: string, formData: FormData, isRetry = false): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${endpoint}`, {
    method: "POST",
    credentials: "include",
    body: formData,
  });

  if (response.status === 401 && !isRetry) {
    try {
      const { authService } = await import("./auth");
      await authService.refresh();
      return fetchMultipart<T>(endpoint, formData, true);
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

  concludeAi: (cardId: string) =>
    fetchAPI<Card>(`/api/cards/${cardId}/conclude-ai`, { method: "POST" }),

  retryAi: (cardId: string) =>
    fetchAPI<Card>(`/api/cards/${cardId}/retry-ai`, { method: "POST" }),

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

  getConflicts: (cardId: string) =>
    fetchAPI<ConflictDetail>(`/api/cards/${cardId}/conflicts`),

  resolveConflicts: (cardId: string, resolutions: FileResolution[]) =>
    fetchAPI<ConflictDetail>(`/api/cards/${cardId}/resolve-conflicts`, {
      method: "POST",
      body: JSON.stringify({ resolutions }),
    }),

  completeMerge: (cardId: string) =>
    fetchAPI<MergeResult>(`/api/cards/${cardId}/complete-merge`, { method: "POST" }),

  abortMerge: (cardId: string) =>
    fetchAPI<void>(`/api/cards/${cardId}/abort-merge`, { method: "POST" }),

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
    fetchAPI<{ messages?: { role: string; content: string | unknown }[] }>(`/api/boards/${boardId}/settings/auto-detect-logs?session_id=${sessionId}`),

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

  getNotifications: (unreadOnly?: boolean) =>
    fetchAPI<Notification[]>(
      `/api/notifications${unreadOnly ? "?unread_only=true" : ""}`
    ),

  markNotificationRead: (id: string) =>
    fetchAPI<Notification>(`/api/notifications/${id}/read`, {
      method: "PATCH",
    }),

  markAllNotificationsRead: () =>
    fetchAPI<void>("/api/notifications/read-all", {
      method: "POST",
    }),

  deleteNotification: (id: string) =>
    fetchAPI<void>(`/api/notifications/${id}`, {
      method: "DELETE",
    }),

  getMe: () => fetchAPI<import("./auth").AuthUser>("/api/auth/me"),

  updateProfile: (data: { nickname?: string; first_name?: string; last_name?: string; email?: string }) =>
    fetchAPI<import("./auth").AuthUser>("/api/auth/me", {
      method: "PATCH",
      body: JSON.stringify(data),
    }),

  changePassword: (data: { current_password: string; new_password: string }) =>
    fetchAPI<{ message: string }>("/api/auth/me/password", {
      method: "PATCH",
      body: JSON.stringify(data),
    }),

  uploadAvatar: (file: File) => {
    const formData = new FormData();
    formData.append("avatar", file);
    return fetchMultipart<import("./auth").AuthUser>("/api/auth/me/avatar", formData);
  },

  getAvatarUrl: (userId: string) => `${API_BASE_URL}/api/users/${userId}/avatar`,
};
