import React, { useEffect, useMemo, useRef, useState } from "react";
import styled from "@emotion/styled";
import { keyframes } from "@emotion/react";
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  Collapse,
  Dialog,
  DialogContent,
  DialogTitle,
  FormControl,
  IconButton,
  InputLabel,
  MenuItem,
  Paper,
  Select,
  Snackbar,
  Tab,
  Tabs,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import {
  AttachFile as AttachFileIcon,
  AutoFixHigh as AutoFixHighIcon,
  Close as CloseIcon,
  ExpandLess as ExpandLessIcon,
  ExpandMore as ExpandMoreIcon,
  FolderOpen as FolderOpenIcon,
  HelpOutline as HelpOutlineIcon,
} from "@mui/icons-material";
import type { BoardSettings, UpdateBoardSettingsRequest } from "../../types/kanban";
import { api } from "../../services/api";
import { useDispatch } from "react-redux";
import { updateBoard } from "../../store/slices/kanbanSlice";
import type { AppDispatch } from "../../redux/store";

interface BoardSettingsDialogProps {
  open: boolean;
  boardId: string;
  boardName: string;
  onClose: () => void;
}

type EditableBoardSettings = Required<UpdateBoardSettingsRequest>;

const EMPTY_SETTINGS: EditableBoardSettings = {
  ai_concurrency: "1",
  codebase_path: "",
  github_repo: "",
  context_markdown: "",
  document_links: "",
  variables: "",
  tech_stack: "",
  communication_patterns: "",
  environments: "",
  code_conventions: "",
  testing_requirements: "",
  api_conventions: "",
  infrastructure: "",
};

const toEditableSettings = (settings: BoardSettings): EditableBoardSettings => ({
  ai_concurrency:
    typeof settings.ai_concurrency === "number" && settings.ai_concurrency >= 0 && settings.ai_concurrency <= 10
      ? String(settings.ai_concurrency)
      : "1",
  codebase_path: settings.codebase_path || "",
  github_repo: settings.github_repo || "",
  context_markdown: settings.context_markdown || "",
  document_links: settings.document_links || "",
  variables: settings.variables || "",
  tech_stack: settings.tech_stack || "",
  communication_patterns: settings.communication_patterns || "",
  environments: settings.environments || "",
  code_conventions: settings.code_conventions || "",
  testing_requirements: settings.testing_requirements || "",
  api_conventions: settings.api_conventions || "",
  infrastructure: settings.infrastructure || "",
});

const miniLarsonSweep = keyframes`
  0%, 100% { left: 0; }
  50% { left: calc(100% - 16px); }
`;

interface MiniLarsonScannerProps {
  scannerColor?: string;
}

const MiniLarsonScanner = styled.div<MiniLarsonScannerProps>`
  position: relative;
  height: 3px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(0, 0, 0, 0.08);
  &::after {
    content: "";
    position: absolute;
    width: 16px;
    height: 100%;
    background: ${(props) => props.scannerColor || "#1565c0"};
    border-radius: 50%;
    box-shadow: 0 0 4px 2px ${(props) => `${props.scannerColor || "#1565c0"}99`};
    animation: ${miniLarsonSweep} 2s ease-in-out infinite;
  }
`;

const FieldLabel: React.FC<{ label: string; tooltip: string }> = ({ label, tooltip }) => (
  <Box sx={{ display: "flex", alignItems: "center", gap: 0.5, mb: 0.5 }}>
    <Typography variant="subtitle2">{label}</Typography>
    <Tooltip title={tooltip} arrow placement="right">
      <HelpOutlineIcon sx={{ fontSize: 16, color: "text.secondary", cursor: "help" }} />
    </Tooltip>
  </Box>
);

export const BoardSettingsDialog: React.FC<BoardSettingsDialogProps> = ({
  open,
  boardId,
  boardName,
  onClose,
}) => {
  const dispatch = useDispatch<AppDispatch>();
  const [tab, setTab] = useState(0);
  const [loading, setLoading] = useState(false);
  const [settings, setSettings] = useState<EditableBoardSettings>(EMPTY_SETTINGS);
  const [saveState, setSaveState] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [snackbarMessage, setSnackbarMessage] = useState<string | null>(null);

  const [editingName, setEditingName] = useState(false);
  const [localBoardName, setLocalBoardName] = useState(boardName);

  useEffect(() => {
    setLocalBoardName(boardName);
  }, [boardName]);

  const handleBoardNameSave = async () => {
    const trimmed = localBoardName.trim();
    if (!trimmed || trimmed === boardName) {
      setLocalBoardName(boardName);
      setEditingName(false);
      return;
    }
    try {
      await dispatch(updateBoard({ id: boardId, data: { name: trimmed } })).unwrap();
      setEditingName(false);
      setSnackbarMessage("Board name updated.");
    } catch {
      setSnackbarMessage("Failed to update board name.");
      setLocalBoardName(boardName);
      setEditingName(false);
    }
  };

  const [autoDetectStatus, setLocalAutoDetectStatus] = useState<"idle" | "running" | "completed" | "failed">("idle");
  const [autoDetectSessionId, setAutoDetectSessionId] = useState<string | null>(null);
  const [autoDetectStartTime, setAutoDetectStartTime] = useState<number | null>(null);
  const [elapsedSeconds, setElapsedSeconds] = useState(0);
  const [showLogs, setShowLogs] = useState(false);
  const [autoDetectLogs, setAutoDetectLogs] = useState<string[]>([]);
  const [clonePath, setClonePath] = useState("");
  const [showPat, setShowPat] = useState(false);
  const [pat, setPat] = useState("");
  const [cloneLoading, setCloneLoading] = useState(false);

  const loadedSettingsRef = useRef<EditableBoardSettings | null>(null);

  useEffect(() => {
    if (!open) {
      return;
    }

    let cancelled = false;

    const loadSettings = async () => {
      try {
        setLoading(true);
        setSaveState("idle");
        const response = await api.getBoardSettings(boardId);
        if (cancelled) {
          return;
        }

        const mapped = toEditableSettings(response);
        loadedSettingsRef.current = mapped;
        setSettings(mapped);

        if (response.auto_detect_status === "running") {
          setLocalAutoDetectStatus("running");
          setAutoDetectSessionId(response.auto_detect_session_id || null);
          if (response.auto_detect_started_at) {
            const started = new Date(response.auto_detect_started_at).getTime();
            setAutoDetectStartTime(started);
            setElapsedSeconds(Math.floor((Date.now() - started) / 1000));
          }
        }
      } catch {
        if (!cancelled) {
          setSaveState("error");
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    loadSettings();

    return () => {
      cancelled = true;
    };
  }, [boardId, open]);

  const settingsHash = useMemo(() => JSON.stringify(settings), [settings]);

  useEffect(() => {
    if (!open || loading || !loadedSettingsRef.current) {
      return;
    }

    const baselineHash = JSON.stringify(loadedSettingsRef.current);
    if (settingsHash === baselineHash) {
      return;
    }

    let cancelled = false;
    const timeout = window.setTimeout(async () => {
      try {
        setSaveState("saving");
        const payload = {
          ...settings,
          ai_concurrency: settings.ai_concurrency ? Number(settings.ai_concurrency) : undefined,
        };
        const updated = await api.updateBoardSettings(boardId, payload);
        if (cancelled) {
          return;
        }

        loadedSettingsRef.current = toEditableSettings(updated);
        setSaveState("saved");
      } catch {
        if (!cancelled) {
          setSaveState("error");
        }
      }
    }, 500);

    return () => {
      cancelled = true;
      window.clearTimeout(timeout);
    };
  }, [boardId, loading, open, settings, settingsHash]);

  useEffect(() => {
    if (autoDetectStatus !== "running" || !autoDetectStartTime) {
      return;
    }
    const interval = setInterval(() => {
      setElapsedSeconds(Math.floor((Date.now() - autoDetectStartTime) / 1000));
    }, 1000);
    return () => clearInterval(interval);
  }, [autoDetectStatus, autoDetectStartTime]);

  useEffect(() => {
    const handler = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail.board_id !== boardId) {
        return;
      }
      if (detail.status === "completed") {
        setLocalAutoDetectStatus("completed");
        api.getBoardSettings(boardId).then((response) => {
          const mapped = toEditableSettings(response);
          loadedSettingsRef.current = mapped;
          setSettings(mapped);
        });
      } else if (detail.status === "failed") {
        setLocalAutoDetectStatus("failed");
      } else if (detail.status === "running") {
        setLocalAutoDetectStatus("running");
        if (detail.session_id) {
          setAutoDetectSessionId(detail.session_id);
        }
        if (!autoDetectStartTime) {
          setAutoDetectStartTime(Date.now());
        }
      }
    };
    window.addEventListener("autoDetectStatus", handler);
    return () => window.removeEventListener("autoDetectStatus", handler);
  }, [autoDetectStartTime, boardId]);

  useEffect(() => {
    if (autoDetectStatus !== "running" || !showLogs || !autoDetectSessionId) {
      return;
    }
    const interval = setInterval(async () => {
      try {
        const data = await api.getAutoDetectLogs(boardId, autoDetectSessionId);
        if (data?.messages) {
          const logs = data.messages
            .filter((m: { role: string; content: string | unknown }) => m.role === "assistant")
            .map((m: { role: string; content: string | unknown }) =>
              typeof m.content === "string" ? m.content : JSON.stringify(m.content)
            );
          setAutoDetectLogs(logs);
        }
      } catch {
      }
    }, 3000);
    return () => clearInterval(interval);
  }, [autoDetectStatus, showLogs, autoDetectSessionId, boardId]);

  const formatElapsed = (seconds: number): string => {
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${m}:${s.toString().padStart(2, "0")}`;
  };

  const handleAutoDetect = async () => {
    if (!settings.codebase_path.trim()) {
      return;
    }

    try {
      const response = await api.autoDetectBoardSettings(boardId, settings.codebase_path);
      setLocalAutoDetectStatus("running");
      setAutoDetectStartTime(Date.now());
      setElapsedSeconds(0);
      setShowLogs(false);
      setAutoDetectLogs([]);
      if (response.session_id) {
        setAutoDetectSessionId(response.session_id);
      }
    } catch {
      setSnackbarMessage("Failed to start auto-detect.");
    }
  };

  const handleCloneRepo = async () => {
    if (!settings.github_repo.trim() || !clonePath.trim()) {
      return;
    }
    const confirmed = window.confirm(`Clone ${settings.github_repo} to ${clonePath}?`);
    if (!confirmed) {
      return;
    }
    setCloneLoading(true);
    try {
      const result = await api.cloneRepo(boardId, settings.github_repo, clonePath, pat || undefined);
      if (result.success) {
        setSettings((prev) => ({ ...prev, codebase_path: result.codebase_path || clonePath }));
        setSnackbarMessage("Repository cloned successfully!");
      } else if (result.error === "auth_required") {
        setShowPat(true);
        setSnackbarMessage("Authentication required. Please provide a Personal Access Token.");
      } else {
        setSnackbarMessage(`Clone failed: ${result.error}`);
      }
    } catch {
      setSnackbarMessage("Failed to clone repository.");
    } finally {
      setCloneLoading(false);
    }
  };

  const handlePickCodebasePath = async () => {
    try {
      const result = await api.pickDirectory();
      if (result.path) {
        setSettings((prev) => ({ ...prev, codebase_path: result.path || "" }));
      }
    } catch {
      setSnackbarMessage("Could not open folder picker.");
    }
  };

  const handlePickClonePath = async () => {
    try {
      const result = await api.pickDirectory();
      if (result.path) {
        setClonePath(result.path);
      }
    } catch {
      setSnackbarMessage("Could not open folder picker.");
    }
  };

  const handlePickReferenceFiles = async () => {
    try {
      const result = await api.pickFiles();
      if (!result.paths || result.paths.length === 0) {
        return;
      }

      setSettings((prev) => {
        const existing = prev.document_links.trim();
        const next = existing ? `${existing}\n${result.paths.join("\n")}` : result.paths.join("\n");
        return { ...prev, document_links: next };
      });
    } catch {
      setSnackbarMessage("Could not open file picker.");
    }
  };

  const updateField = (key: keyof EditableBoardSettings, value: string | number) => {
    setSettings((prev) => ({ ...prev, [key]: value }));
  };

  const saveStatusLabel =
    saveState === "saving"
      ? "Saving..."
      : saveState === "saved"
      ? "Saved"
      : saveState === "error"
      ? "Save failed"
      : "";

  return (
    <>
      <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
        <DialogTitle sx={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <Box>
            <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
              {editingName ? (
                <TextField
                  value={localBoardName}
                  onChange={(e) => setLocalBoardName(e.target.value)}
                  onBlur={handleBoardNameSave}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleBoardNameSave();
                    if (e.key === "Escape") {
                      setLocalBoardName(boardName);
                      setEditingName(false);
                    }
                  }}
                  variant="standard"
                  autoFocus
                  inputProps={{ style: { fontSize: "1.25rem", fontWeight: 500 } }}
                  sx={{ minWidth: 200 }}
                />
              ) : (
                <Typography
                  variant="h6"
                  onClick={() => setEditingName(true)}
                  sx={{ cursor: "pointer", "&:hover": { textDecoration: "underline dotted", textUnderlineOffset: 4 } }}
                  title="Click to rename board"
                >
                  {localBoardName} Settings
                </Typography>
              )}
              <Tooltip
                title="Everything you fill in here becomes shared context for AI when it works on any card in this board. Think of it as giving AI a project briefing - it reads this before starting any task. Your AI provider (Anthropic, OpenAI, Gemini) automatically caches this context, so it only costs full price once, then up to 90% cheaper on repeat use."
                arrow
                placement="bottom"
              >
                <HelpOutlineIcon sx={{ fontSize: 18, color: "text.secondary", cursor: "help" }} />
              </Tooltip>
            </Box>
            {saveStatusLabel && (
              <Typography variant="caption" color={saveState === "error" ? "error" : "text.secondary"}>
                {saveStatusLabel}
              </Typography>
            )}
          </Box>
          <IconButton onClick={onClose}>
            <CloseIcon />
          </IconButton>
        </DialogTitle>

        <DialogContent dividers>
          {loading ? (
            <Box sx={{ py: 6, display: "flex", justifyContent: "center" }}>
              <CircularProgress size={28} />
            </Box>
          ) : (
            <>
              <Alert severity="info" sx={{ mb: 2 }}>
                Board settings are your shared project briefing for AI. The clearer this is, the more accurate and consistent AI output will be across every card.
              </Alert>

              <Tabs value={tab} onChange={(_, value) => setTab(value)} sx={{ mb: 3 }}>
                <Tab
                  icon={<AutoFixHighIcon fontSize="small" />}
                  iconPosition="start"
                  label="Auto-Detect"
                  sx={{ color: "#1565c0", "&.Mui-selected": { color: "#1565c0" } }}
                />
                <Tab label="Basic" />
                <Tab label="Technical" />
                <Tab label="Conventions" />
              </Tabs>

              {tab === 0 && (
                <Box sx={{ display: "grid", gap: 2.5 }}>
                  <Box sx={{ p: 2, borderRadius: 1, bgcolor: "action.hover", border: "1px solid", borderColor: "divider" }}>
                    <Typography variant="body1" color="text.secondary">
                      Tired of typing all details? Let AI analyze your codebase and fill everything in.
                    </Typography>
                  </Box>

                  <Box>
                    <FieldLabel
                      label="Codebase Path"
                      tooltip="The folder path where your project code lives on the server. AI uses this to know where to look for files and run commands. Example: /home/user/projects/my-saas-app"
                    />
                    <Box sx={{ display: "flex", gap: 1, alignItems: "flex-start" }}>
                      <TextField
                        fullWidth
                        label="Codebase Path"
                        value={settings.codebase_path}
                        onChange={(event) => updateField("codebase_path", event.target.value)}
                        placeholder="/home/user/projects/my-app"
                        InputLabelProps={{ shrink: true }}
                      />
                      <IconButton onClick={handlePickCodebasePath} sx={{ mt: 1 }}>
                        <FolderOpenIcon />
                      </IconButton>
                    </Box>
                  </Box>

                  <Box>
                    <FieldLabel
                      label="GitHub Repository"
                      tooltip="Link to your project's GitHub repository. Optionally clone it directly, then run auto-detect against the cloned path."
                    />
                    <TextField
                      fullWidth
                      label="GitHub URL"
                      value={settings.github_repo}
                      onChange={(event) => updateField("github_repo", event.target.value)}
                      placeholder="https://github.com/org/repo"
                      InputLabelProps={{ shrink: true }}
                    />

                    {settings.github_repo.trim() && (
                      <Box
                        sx={{
                          mt: 1.5,
                          display: "grid",
                          gap: 1.5,
                          pl: 2,
                          borderLeft: "2px solid",
                          borderColor: "divider",
                        }}
                      >
                        <Box sx={{ display: "flex", gap: 1, alignItems: "flex-start" }}>
                          <TextField
                            fullWidth
                            size="small"
                            label="Clone to (local path)"
                            value={clonePath}
                            onChange={(event) => setClonePath(event.target.value)}
                            placeholder="/home/user/projects/cloned-repo"
                            InputLabelProps={{ shrink: true }}
                          />
                          <IconButton onClick={handlePickClonePath} size="small">
                            <FolderOpenIcon />
                          </IconButton>
                        </Box>

                        {showPat && (
                          <TextField
                            fullWidth
                            size="small"
                            type="password"
                            label="Personal Access Token"
                            value={pat}
                            onChange={(event) => setPat(event.target.value)}
                            helperText="Required for private repos. Generate at github.com/settings/tokens"
                            InputLabelProps={{ shrink: true }}
                          />
                        )}

                        {!showPat && (
                          <Typography
                            variant="caption"
                            sx={{ cursor: "pointer", color: "primary.main", "&:hover": { textDecoration: "underline" } }}
                            onClick={() => setShowPat(true)}
                          >
                            Private repo? Click to add authentication
                          </Typography>
                        )}

                        <Button
                          variant="outlined"
                          size="small"
                          onClick={handleCloneRepo}
                          disabled={!clonePath.trim() || cloneLoading}
                          startIcon={cloneLoading ? <CircularProgress size={16} /> : undefined}
                        >
                          {cloneLoading ? "Cloning..." : "Clone Repository"}
                        </Button>
                      </Box>
                    )}
                  </Box>

                  <Button
                    variant="contained"
                    fullWidth
                    size="large"
                    startIcon={<AutoFixHighIcon />}
                    onClick={handleAutoDetect}
                    disabled={!settings.codebase_path.trim() || autoDetectStatus === "running"}
                    sx={{ bgcolor: "#1565c0", "&:hover": { bgcolor: "#0d47a1" }, py: 1.5, fontSize: "1rem" }}
                  >
                    {autoDetectStatus === "running" ? "Analysis in Progress..." : "Auto-Detect Codebase"}
                  </Button>

                  {autoDetectStatus !== "idle" && (
                    <Box sx={{ display: "grid", gap: 1.5 }}>
                      {autoDetectStatus === "running" && <MiniLarsonScanner scannerColor="#1565c0" />}

                      <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                        <Typography variant="body2" color={autoDetectStatus === "failed" ? "error" : "text.secondary"}>
                          {autoDetectStatus === "running"
                            ? "AI is analyzing your codebase..."
                            : autoDetectStatus === "completed"
                            ? "Analysis complete! Settings have been updated."
                            : "Analysis failed. Please try again."}
                        </Typography>
                        {(autoDetectStatus === "running" || autoDetectStatus === "completed") && (
                          <Typography variant="body2" color="text.secondary" sx={{ fontFamily: "monospace" }}>
                            {formatElapsed(elapsedSeconds)}
                          </Typography>
                        )}
                      </Box>

                      {autoDetectSessionId && (
                        <>
                          <Button
                            size="small"
                            variant="text"
                            onClick={() => setShowLogs(!showLogs)}
                            endIcon={showLogs ? <ExpandLessIcon /> : <ExpandMoreIcon />}
                            sx={{ justifyContent: "flex-start" }}
                          >
                            {showLogs ? "Hide AI Logs" : "Show AI Logs"}
                          </Button>
                          <Collapse in={showLogs}>
                            <Paper
                              sx={{
                                p: 1.5,
                                bgcolor: "#1a1a2e",
                                maxHeight: 300,
                                overflow: "auto",
                                fontFamily: "monospace",
                                fontSize: "0.8rem",
                                color: "#e0e0e0",
                              }}
                            >
                              {autoDetectLogs.length === 0 ? (
                                <Typography variant="body2" sx={{ color: "#666" }}>
                                  Waiting for logs...
                                </Typography>
                              ) : (
                                autoDetectLogs.map((log, i) => (
                                  <Box key={i} sx={{ mb: 0.5, whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
                                    {log}
                                  </Box>
                                ))
                              )}
                            </Paper>
                          </Collapse>
                        </>
                      )}
                    </Box>
                  )}
                </Box>
              )}

              {tab === 1 && (
                <Box sx={{ display: "grid", gap: 2 }}>
                  <Box>
                    <FieldLabel
                      label="AI Concurrency"
                      tooltip="How many AI jobs can run in parallel on this board. Higher values increase throughput but may hit API rate limits."
                    />
                    <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
                      <FormControl size="small" sx={{ minWidth: 220 }}>
                        <InputLabel id="ai-concurrency-label">AI Concurrency</InputLabel>
                        <Select
                          labelId="ai-concurrency-label"
                          label="AI Concurrency"
                          value={settings.ai_concurrency}
                          onChange={(event) => updateField("ai_concurrency", event.target.value)}
                        >
                          {Array.from({ length: 10 }, (_, index) => {
                            const value = String(index + 1);
                            return (
                              <MenuItem key={value} value={value}>
                                {value}
                              </MenuItem>
                            );
                          })}
                          <MenuItem value="0">Unlimited</MenuItem>
                        </Select>
                      </FormControl>
                      <Tooltip title="Unlimited can trigger API rate limits quickly. Prefer 1-3 unless you have high quotas." arrow>
                        <HelpOutlineIcon sx={{ fontSize: 18, color: "warning.main", cursor: "help" }} />
                      </Tooltip>
                    </Box>
                  </Box>

                  <Box>
                    <FieldLabel
                      label="Context (Free-form Notes)"
                      tooltip="Write anything you want AI to always keep in mind when working on this board. This is free-form - write it like you're briefing a new team member. For example: 'Our app has a checkout service that handles payments. Never create new services for payment-related features - use the existing checkout service.' This text is included as-is in every AI prompt."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={6}
                      label="Context (Free-form Notes)"
                      value={settings.context_markdown}
                      onChange={(event) => updateField("context_markdown", event.target.value)}
                      placeholder={
                        "Write project context here...\n\nExample:\n- Our microservice architecture has 12 services\n- The checkout-service handles all payment logic\n- Never create new services for existing functionality"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>

                  <Box>
                    <FieldLabel
                      label="Reference Documents"
                      tooltip="Files or documents that AI should reference when working on cards. These could be architecture docs, API specs, design documents, or any file that helps AI understand your project. AI will read these before starting work."
                    />
                    <Box sx={{ display: "grid", gap: 1 }}>
                      <TextField
                        fullWidth
                        multiline
                        rows={3}
                        label="Reference Documents"
                        value={settings.document_links}
                        onChange={(event) => updateField("document_links", event.target.value)}
                        placeholder={"One file path or URL per line\n\nExamples:\n/docs/architecture.md\nhttps://wiki.internal/api-guide"}
                        InputLabelProps={{ shrink: true }}
                      />
                      <Box sx={{ display: "flex", justifyContent: "flex-start" }}>
                        <Button variant="outlined" startIcon={<AttachFileIcon />} onClick={handlePickReferenceFiles}>
                          Browse Files
                        </Button>
                      </Box>
                    </Box>
                  </Box>

                  <Box>
                    <FieldLabel
                      label="Environment Variables"
                      tooltip="Key-value pairs that AI should know about your project. These are NOT secrets - don't put passwords here. Use this for URLs, port numbers, service names, or any project constants that AI needs when writing code or configs."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={3}
                      label="Environment Variables"
                      value={settings.variables}
                      onChange={(event) => updateField("variables", event.target.value)}
                      placeholder={
                        "One per line in KEY=VALUE format\n\nExamples:\nSTAGING_URL=https://staging.example.com\nGRPC_PORT=50051\nDEFAULT_DB=postgresql"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>
                </Box>
              )}

              {tab === 2 && (
                <Box sx={{ display: "grid", gap: 2 }}>
                  <Box>
                    <FieldLabel
                      label="Tech Stack"
                      tooltip="List the programming languages, frameworks, databases, and tools your project uses, including version numbers. AI uses this to write code in the correct language and version, and to suggest compatible libraries."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={3}
                      label="Tech Stack"
                      value={settings.tech_stack}
                      onChange={(event) => updateField("tech_stack", event.target.value)}
                      placeholder={
                        "Examples:\n- Go 1.22 (backend services)\n- React 18 + TypeScript (frontend)\n- PostgreSQL 16 (main database)\n- Redis 7 (caching)\n- gRPC (service communication)"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>

                  <Box>
                    <FieldLabel
                      label="Communication Patterns"
                      tooltip="How do your services talk to each other? AI needs this to write correct integration code and avoid creating incompatible communication methods."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={3}
                      label="Communication Patterns"
                      value={settings.communication_patterns}
                      onChange={(event) => updateField("communication_patterns", event.target.value)}
                      placeholder={
                        "Examples:\n- Internal services: gRPC on port 50051\n- External APIs: REST with JSON\n- Async events: Kafka topics\n- Service discovery: Consul"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>

                  <Box>
                    <FieldLabel
                      label="Environments"
                      tooltip="List your deployment environments and their details. AI uses this to write correct configuration, deployment scripts, and environment-specific code."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={3}
                      label="Environments"
                      value={settings.environments}
                      onChange={(event) => updateField("environments", event.target.value)}
                      placeholder={
                        "Examples:\n- Dev: localhost:8080, local PostgreSQL\n- Staging: staging.example.com, shared DB\n- Production: api.example.com, replicated DB cluster"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>

                  <Box>
                    <FieldLabel
                      label="Infrastructure"
                      tooltip="Describe your deployment infrastructure, CI/CD pipeline, and DevOps setup. AI uses this to write correct Dockerfiles, CI configs, and deployment scripts."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={3}
                      label="Infrastructure"
                      value={settings.infrastructure}
                      onChange={(event) => updateField("infrastructure", event.target.value)}
                      placeholder={
                        "Examples:\n- Kubernetes on AWS EKS\n- GitHub Actions for CI/CD\n- ArgoCD for GitOps deployment\n- Docker with multi-stage builds"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>
                </Box>
              )}

              {tab === 3 && (
                <Box sx={{ display: "grid", gap: 2 }}>
                  <Box>
                    <FieldLabel
                      label="Code Conventions"
                      tooltip="Your team's coding standards and rules. AI will follow these conventions when writing code, so the output matches your existing codebase style."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={3}
                      label="Code Conventions"
                      value={settings.code_conventions}
                      onChange={(event) => updateField("code_conventions", event.target.value)}
                      placeholder={
                        "Examples:\n- Go standard project layout\n- ESLint + Prettier for frontend\n- snake_case for DB columns, camelCase for Go/TS\n- All errors must be wrapped with context"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>

                  <Box>
                    <FieldLabel
                      label="Testing Requirements"
                      tooltip="Your team's testing standards. AI will write tests that meet these requirements and follow your existing test patterns."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={3}
                      label="Testing Requirements"
                      value={settings.testing_requirements}
                      onChange={(event) => updateField("testing_requirements", event.target.value)}
                      placeholder={
                        "Examples:\n- Minimum 80% code coverage\n- Integration tests with testcontainers\n- Jest + React Testing Library for frontend\n- Table-driven tests for Go"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>

                  <Box>
                    <FieldLabel
                      label="API Conventions"
                      tooltip="Your API design rules and standards. AI will follow these when creating new endpoints or modifying existing ones."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={3}
                      label="API Conventions"
                      value={settings.api_conventions}
                      onChange={(event) => updateField("api_conventions", event.target.value)}
                      placeholder={
                        "Examples:\n- RESTful with JSON:API envelope\n- JWT auth via Authorization header\n- Standard error format: { error: { code, message } }\n- API versioning via URL path (/v1/, /v2/)"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>
                </Box>
              )}
            </>
          )}
        </DialogContent>
      </Dialog>

      <Snackbar
        open={Boolean(snackbarMessage)}
        autoHideDuration={3000}
        onClose={() => setSnackbarMessage(null)}
        anchorOrigin={{ vertical: "bottom", horizontal: "center" }}
      >
        <Alert severity="info" onClose={() => setSnackbarMessage(null)}>
          {snackbarMessage}
        </Alert>
      </Snackbar>
    </>
  );
};

export default BoardSettingsDialog;
