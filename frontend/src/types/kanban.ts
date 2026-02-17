export type Stage = "backlog" | "plan" | "todo" | "in_progress" | "review" | "done";

export interface Subtask {
  id: string;
  card_id: string;
  title: string;
  completed: boolean;
  position: number;
  created_at: string;
  updated_at: string;
}

export interface Label {
  id: string;
  name: string;
  color: string;
}

export interface Comment {
  id: string;
  card_id: string;
  author: string;
  content: string;
  user_id?: string;
  created_at: string;
}

export interface Card {
  id: string;
  title: string;
  description: string;
  stage: Stage;
  position: number;
  priority: string;
  working_directory: string;
  ai_session_id: string | null;
  ai_status: string;
  ai_progress: string;
  plan_path: string | null;
  linked_documents: string;
  branch_name: string;
  worktree_path: string;
  created_at: string;
  updated_at: string;
  ai_agent: string | null;
  subtask_count: number;
  subtask_completed: number;
  label_count: number;
  comment_count: number;
}

export interface BoardResponse {
  backlog: Card[];
  plan: Card[];
  todo: Card[];
  in_progress: Card[];
  review: Card[];
  done: Card[];
}

export interface FileDiff {
  path: string;
  status: string;
  additions: number;
  deletions: number;
  diff: string;
}

export interface DiffResult {
  files: FileDiff[];
  stats: {
    files_changed: number;
    additions: number;
    deletions: number;
  };
}

export interface MergeResult {
  success: boolean;
  message: string;
  conflicts: string[];
  conflict_detail?: ConflictDetail;
}

export interface ConflictDetail {
  files: ConflictFile[];
  merge_in_progress: boolean;
}

export interface FileResolution {
  file_path: string;
  choice: string;
  manual_content?: string;
}

export interface ResolveRequest {
  resolutions: FileResolution[];
}

export interface CreateCardRequest {
  title: string;
  description?: string;
  stage?: Stage;
  priority?: string;
  working_directory?: string;
  board_id?: string;
}

export interface UpdateCardRequest {
  title?: string;
  description?: string;
  priority?: string;
  working_directory?: string;
  linked_documents?: string;
  ai_agent?: string | null;
}

export interface MoveCardRequest {
  stage: Stage;
  position: number;
}

export interface CreateSubtaskRequest {
  title: string;
}

export interface UpdateSubtaskRequest {
  title?: string;
  completed?: boolean;
}

export interface CreateCommentRequest {
  author: string;
  content: string;
}

export interface Board {
  id: string;
  name: string;
  position: number;
  created_at: string;
  updated_at: string;
}

export interface ReorderBoardRequest {
  position: number;
}

export interface CreateBoardRequest {
  name: string;
}

export interface UpdateBoardRequest {
  name: string;
}

export interface AgentLog {
  id: string;
  card_id: string;
  session_id: string;
  event_type: string;
  agent: string | null;
  content: string;
  metadata: string;
  created_at: string;
}

export interface SessionMapping {
  child_session_id: string;
  card_id: string;
  parent_session_id: string;
  agent_type: string | null;
  description: string;
  created_at: string;
}

export interface AgentActivityEntry {
  agent_type: string | null;
  event_count: number;
  first_seen: string;
  last_seen: string;
}

export interface AgentActivityResponse {
  card_id: string;
  agents: AgentActivityEntry[];
  session_mappings: SessionMapping[];
}

export interface CardVersion {
  id: string;
  card_id: string;
  snapshot: string;
  changed_by: string;
  created_at: string;
}

export interface BoardSettings {
  board_id: string;
  ai_concurrency: number;
  codebase_path: string;
  github_repo: string;
  auto_detect_status: string;
  auto_detect_session_id: string;
  auto_detect_started_at: string;
  context_markdown: string;
  document_links: string;
  variables: string;
  tech_stack: string;
  communication_patterns: string;
  environments: string;
  code_conventions: string;
  testing_requirements: string;
  api_conventions: string;
  infrastructure: string;
  created_at: string;
  updated_at: string;
}

export type NotificationType = "card_stage_changed" | "ai_completed" | "ai_question_pending" | "review_requested" | "ai_error";

export interface Notification {
  id: string;
  user_id: string | null;
  notification_type: NotificationType;
  title: string;
  message: string;
  card_id: string | null;
  board_id: string | null;
  is_read: boolean;
  created_at: string;
}

export interface UpdateBoardSettingsRequest {
  ai_concurrency?: number | string;
  codebase_path?: string;
  github_repo?: string;
  context_markdown?: string;
  document_links?: string;
  variables?: string;
  tech_stack?: string;
  communication_patterns?: string;
  environments?: string;
  code_conventions?: string;
  testing_requirements?: string;
  api_conventions?: string;
  infrastructure?: string;
}

export type ConflictType = "both-modified" | "deleted-by-us" | "deleted-by-them" | "added-by-both";

export interface ConflictFile {
  path: string;
  ours_content: string | null;
  theirs_content: string | null;
  base_content: string | null;
  conflict_type: ConflictType;
  is_binary: boolean;
}

export interface ConflictDetail {
  files: ConflictFile[];
  merge_in_progress: boolean;
}
