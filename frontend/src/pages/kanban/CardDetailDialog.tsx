import React, { useState, useEffect, useMemo } from "react";
import { useSelector, useDispatch } from "react-redux";
import styled from "@emotion/styled";
import { keyframes } from "@emotion/react";
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  Button,
  IconButton,
  Chip,
  List,
  ListItem,
  ListItemText,
  Checkbox,
  Select,
  MenuItem,
  FormControl,
  InputLabel,
  LinearProgress,
  Typography,
  Box,
  Divider,
  Paper,
  CircularProgress,
} from "@mui/material";
import {
  Close as CloseIcon,
  Delete as DeleteIcon,
  Edit as EditIcon,
  Check as CheckIcon,
  Add as AddIcon,
  AutoFixHigh as AutoFixHighIcon,
  DragIndicator as DragIndicatorIcon,
  AttachFile as AttachFileIcon,
  CloudUpload as CloudUploadIcon,
  StopCircle as StopCircleIcon,
} from "@mui/icons-material";
import {
  DndContext,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
  DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { RootState, AppDispatch } from "../../redux/store";
import type { CardVersion } from "../../types/kanban";
import { updateCard, deleteCard, fetchBoard } from "../../store/slices/kanbanSlice";
import { AgentLogViewer } from "./AgentLogViewer";
import { api } from "../../services/api";

const API_BASE_URL = import.meta.env.VITE_API_URL || `${window.location.protocol}//${window.location.hostname}:3000`;

const DialogHeader = styled(DialogTitle)`
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: ${(props) => props.theme.spacing(3)};
`;

const Section = styled(Box)`
  margin-bottom: ${(props) => props.theme.spacing(4)};
`;

const SectionTitle = styled(Typography)`
  font-weight: 600;
  margin-bottom: ${(props) => props.theme.spacing(2)};
`;

const miniLarsonSweep = keyframes`
  0%, 100% { left: 0; }
  50% { left: calc(100% - 16px); }
`;

const MiniLarsonScanner = styled.div`
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 2px;
  overflow: hidden;
  border-radius: 0 0 4px 4px;
  &::after {
    content: '';
    position: absolute;
    width: 16px;
    height: 100%;
    background: #ff3300;
    border-radius: 50%;
    box-shadow: 0 0 4px 2px rgba(255, 51, 0, 0.6), 0 0 8px 4px rgba(255, 51, 0, 0.3);
    animation: ${miniLarsonSweep} 2s ease-in-out infinite;
  }
`;

interface CardFile {
  id: string;
  card_id: string;
  filename: string;
  original_filename: string;
  file_size: number;
  mime_type: string;
  uploaded_at: string;
}

interface SubtaskItem {
  id: string;
  card_id: string;
  title: string;
  completed: boolean;
  position: number;
  phase: string;
  phase_order: number;
}

interface CardSnapshot {
  title?: string;
  description?: string;
  priority?: string;
  stage?: string;
  working_directory?: string;
  linked_documents?: string;
}

const SortableSubtask: React.FC<{
  subtask: SubtaskItem;
  taskNumber: number;
  editingId: string | null;
  editingTitle: string;
  onEditStart: (id: string, title: string) => void;
  onEditSave: (id: string) => void;
  onEditChange: (val: string) => void;
  onEditCancel: () => void;
  onToggle: (id: string, completed: boolean) => void;
  onDelete: (id: string) => void;
}> = ({ subtask, taskNumber, editingId, editingTitle, onEditStart, onEditSave, onEditChange, onEditCancel, onToggle, onDelete }) => {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: subtask.id });
  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  return (
    <ListItem ref={setNodeRef} style={style} secondaryAction={
      <Box>
        {editingId === subtask.id ? (
          <IconButton edge="end" onClick={() => onEditSave(subtask.id)}><CheckIcon /></IconButton>
        ) : (
          <IconButton edge="end" onClick={() => onEditStart(subtask.id, subtask.title)}><EditIcon /></IconButton>
        )}
        <IconButton edge="end" onClick={() => onDelete(subtask.id)}><DeleteIcon /></IconButton>
      </Box>
    }>
      <IconButton size="small" sx={{ cursor: 'grab', mr: 0.5 }} {...attributes} {...listeners}>
        <DragIndicatorIcon fontSize="small" />
      </IconButton>
      <Checkbox checked={subtask.completed} onChange={(e) => onToggle(subtask.id, e.target.checked)} />
      {editingId === subtask.id ? (
        <TextField
          size="small"
          fullWidth
          value={editingTitle}
          onChange={(e) => onEditChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") onEditSave(subtask.id);
            if (e.key === "Escape") onEditCancel();
          }}
          autoFocus
        />
      ) : (
        <ListItemText primary={`Task ${taskNumber}: ${subtask.title}`} sx={{ textDecoration: subtask.completed ? 'line-through' : 'none', color: subtask.completed ? 'text.secondary' : 'text.primary' }} />
      )}
    </ListItem>
  );
};

interface CardDetailDialogProps {
  open: boolean;
  onClose: () => void;
  cardId: string;
}

export const CardDetailDialog: React.FC<CardDetailDialogProps> = ({ open, onClose, cardId }) => {
  const columns = useSelector((state: RootState) => state.kanban.columns);
  const activeBoardId = useSelector((state: RootState) => state.kanban.activeBoardId);
  const card = Object.values(columns)
    .flat()
    .find((c) => c.id === cardId);

  const dispatch = useDispatch<AppDispatch>();
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [priority, setPriority] = useState("medium");
  const [aiAgent, setAiAgent] = useState("");
  const [workingDir, setWorkingDir] = useState(".");
  const [newSubtaskByPhase, setNewSubtaskByPhase] = useState<Record<string, string>>({});
  const [newComment, setNewComment] = useState("");
  const [linkedDocs, setLinkedDocs] = useState("");
  const [subtasks, setSubtasks] = useState<SubtaskItem[]>([]);
  const [newPhaseName, setNewPhaseName] = useState("");
  const [comments, setComments] = useState<any[]>([]);
  const [files, setFiles] = useState<CardFile[]>([]);
  const [uploading, setUploading] = useState(false);
  const [editingSubtaskId, setEditingSubtaskId] = useState<string | null>(null);
  const [editingSubtaskTitle, setEditingSubtaskTitle] = useState("");
  const [editingCommentId, setEditingCommentId] = useState<string | null>(null);
  const [editingCommentContent, setEditingCommentContent] = useState("");
  const [editingPhaseName, setEditingPhaseName] = useState<string | null>(null);
  const [editingPhaseValue, setEditingPhaseValue] = useState("");
  const [versions, setVersions] = useState<CardVersion[]>([]);
  const [showVersions, setShowVersions] = useState(false);

  useEffect(() => {
    if (card) {
      setVersions([]);
      setShowVersions(false);
      setTitle(card.title);
      setDescription(card.description || "");
      setPriority(card.priority);
      setAiAgent(card.ai_agent || "");
      setWorkingDir(card.working_directory || ".");
      setLinkedDocs(card.linked_documents || "");
      
      fetch(`${API_BASE_URL}/api/cards/${card.id}/files`)
        .then((res) => res.json())
        .then((data) => setFiles(data))
        .catch((err) => console.error("Failed to fetch files:", err));
      
      fetch(`${API_BASE_URL}/api/cards/${card.id}/comments`)
        .then((res) => res.json())
        .then((data) => setComments(data))
        .catch((err) => console.error("Failed to fetch comments:", err));
      
      fetch(`${API_BASE_URL}/api/cards/${card.id}`)
        .then((res) => res.json())
        .then((data) => {
          setSubtasks(data.subtasks || []);
        })
        .catch((err) => console.error("Failed to fetch card details:", err));
    }
  }, [card?.id]);

  const handleToggleVersions = async () => {
    if (!card) return;
    if (!showVersions) {
      try {
        const data = await api.getCardVersions(card.id);
        setVersions(data);
      } catch (err) {
        console.error("Failed to fetch card versions:", err);
      }
    }
    setShowVersions(!showVersions);
  };

  const handleRestore = async (versionId: string) => {
    if (!card || !window.confirm("Restore this version? Current state will be saved as a new version.")) {
      return;
    }

    try {
      await api.restoreCardVersion(card.id, versionId);
      const data = await api.getCardVersions(card.id);
      setVersions(data);
      dispatch(fetchBoard(activeBoardId || undefined));
    } catch (err) {
      console.error("Failed to restore card version:", err);
    }
  };

  const handleFileUpload = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const selectedFiles = event.target.files;
    if (!selectedFiles || !card) return;

    setUploading(true);
    const formData = new FormData();
    Array.from(selectedFiles).forEach((file) => {
      formData.append("file", file);
    });

    try {
       const response = await fetch(`${API_BASE_URL}/api/cards/${card.id}/files`, {
        method: "POST",
        body: formData,
      });
      const uploadedFiles = await response.json();
      setFiles([...files, ...uploadedFiles]);
    } catch (err) {
      console.error("Failed to upload files:", err);
    } finally {
      setUploading(false);
    }
  };

  const handleFileDelete = async (fileId: string) => {
    try {
       await fetch(`${API_BASE_URL}/api/files/${fileId}`, {
        method: "DELETE",
      });
      setFiles(files.filter((f) => f.id !== fileId));
    } catch (err) {
      console.error("Failed to delete file:", err);
    }
  };

  const handleFileDownload = (fileId: string) => {
    window.open(`${API_BASE_URL}/api/files/${fileId}`, "_blank");
  };

  const subtaskSensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 5 } }));

  const phases = useMemo(() => {
    const phaseMap = new Map<string, { order: number; tasks: SubtaskItem[] }>();
    for (const st of subtasks) {
      const key = st.phase || "Phase 1";
      if (!phaseMap.has(key)) {
        phaseMap.set(key, { order: st.phase_order || 1, tasks: [] });
      }
      phaseMap.get(key)!.tasks.push(st);
    }
    return Array.from(phaseMap.entries())
      .sort(([, a], [, b]) => a.order - b.order)
      .map(([name, data]) => ({ name, order: data.order, tasks: data.tasks }));
  }, [subtasks]);

  const handleAddSubtask = async (phase: string, phaseOrder: number) => {
    const text = newSubtaskByPhase[phase] || "";
    if (!text.trim() || !card) return;
    try {
       const response = await fetch(`${API_BASE_URL}/api/cards/${card.id}/subtasks`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ title: text, phase, phase_order: phaseOrder }),
      });
      const subtask = await response.json();
      setSubtasks([...subtasks, subtask]);
      setNewSubtaskByPhase({ ...newSubtaskByPhase, [phase]: "" });
    } catch (err) {
      console.error("Failed to add subtask:", err);
    }
  };

  const handleAddPhase = () => {
    const name = newPhaseName.trim();
    if (!name) return;
    const maxOrder = phases.length > 0 ? Math.max(...phases.map(p => p.order)) : 0;
    setSubtasks([...subtasks]);
    setNewPhaseName("");
    setSubtasks(prev => {
      const temp: SubtaskItem = { id: `_placeholder_${Date.now()}`, card_id: card?.id || "", title: "", completed: false, position: 0, phase: name, phase_order: maxOrder + 1 };
      return [...prev, temp];
    });
  };

  const handleRenamephase = async (oldName: string, newName: string) => {
    const trimmed = newName.trim();
    if (!trimmed || trimmed === oldName) {
      setEditingPhaseName(null);
      return;
    }
    const phaseSubtasks = subtasks.filter(s => s.phase === oldName && !s.id.startsWith("_placeholder_"));
    const updated = subtasks.map(s => s.phase === oldName ? { ...s, phase: trimmed } : s);
    setSubtasks(updated);
    setEditingPhaseName(null);

    for (const st of phaseSubtasks) {
      try {
         await fetch(`${API_BASE_URL}/api/subtasks/${st.id}`, {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ phase: trimmed }),
        });
      } catch (err) {
        console.error("Failed to rename phase:", err);
      }
    }
  };

  const handleSubtaskDragEnd = async (event: DragEndEvent, phaseName: string) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const phaseTasks = subtasks.filter(s => s.phase === phaseName);
    const oldIndex = phaseTasks.findIndex(s => s.id === active.id);
    const newIndex = phaseTasks.findIndex(s => s.id === over.id);
    if (oldIndex === -1 || newIndex === -1) return;

    const reordered = [...phaseTasks];
    const [moved] = reordered.splice(oldIndex, 1);
    reordered.splice(newIndex, 0, moved);

    const updatedSubtasks = subtasks.map(s => {
      if (s.phase !== phaseName) return s;
      const idx = reordered.findIndex(r => r.id === s.id);
      return { ...s, position: (idx + 1) * 1000 };
    });
    setSubtasks(updatedSubtasks);

    for (let i = 0; i < reordered.length; i++) {
      if (reordered[i].id.startsWith("_placeholder_")) continue;
      try {
         await fetch(`${API_BASE_URL}/api/subtasks/${reordered[i].id}`, {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ position: (i + 1) * 1000 }),
        });
      } catch (err) {
        console.error("Failed to reorder subtask:", err);
      }
    }
  };

  const handleToggleSubtask = async (subtaskId: string, completed: boolean) => {
    try {
       await fetch(`${API_BASE_URL}/api/subtasks/${subtaskId}`, {
         method: "PATCH",
         headers: { "Content-Type": "application/json" },
         body: JSON.stringify({ completed }),
       });
      setSubtasks(subtasks.map((s) => s.id === subtaskId ? { ...s, completed } : s));
    } catch (err) {
      console.error("Failed to toggle subtask:", err);
    }
  };

  const handleDeleteSubtask = async (subtaskId: string) => {
    try {
       await fetch(`${API_BASE_URL}/api/subtasks/${subtaskId}`, {
         method: "DELETE",
       });
      setSubtasks(subtasks.filter((s) => s.id !== subtaskId));
    } catch (err) {
      console.error("Failed to delete subtask:", err);
    }
  };

  const handleEditSubtask = async (subtaskId: string) => {
    if (!editingSubtaskTitle.trim()) return;
    try {
       const response = await fetch(`${API_BASE_URL}/api/subtasks/${subtaskId}`, {
         method: "PATCH",
         headers: { "Content-Type": "application/json" },
         body: JSON.stringify({ title: editingSubtaskTitle }),
       });
      const updated = await response.json();
      setSubtasks(subtasks.map((s) => s.id === subtaskId ? updated : s));
      setEditingSubtaskId(null);
      setEditingSubtaskTitle("");
    } catch (err) {
      console.error("Failed to edit subtask:", err);
    }
  };

  const handleEditComment = async (commentId: string) => {
    if (!editingCommentContent.trim()) return;
    try {
       const response = await fetch(`${API_BASE_URL}/api/comments/${commentId}`, {
         method: "PATCH",
         headers: { "Content-Type": "application/json" },
         body: JSON.stringify({ content: editingCommentContent }),
       });
      const updated = await response.json();
      setComments(comments.map((c) => c.id === commentId ? updated : c));
      setEditingCommentId(null);
      setEditingCommentContent("");
    } catch (err) {
      console.error("Failed to edit comment:", err);
    }
  };

  const handleDeleteComment = async (commentId: string) => {
    try {
       await fetch(`${API_BASE_URL}/api/comments/${commentId}`, { method: "DELETE" });
      setComments(comments.filter((c) => c.id !== commentId));
    } catch (err) {
      console.error("Failed to delete comment:", err);
    }
  };

  const handleAddComment = async () => {
    if (!newComment.trim() || !card) return;
    try {
       const response = await fetch(`${API_BASE_URL}/api/cards/${card.id}/comments`, {
         method: "POST",
         headers: { "Content-Type": "application/json" },
         body: JSON.stringify({ author: "User", content: newComment }),
       });
      const comment = await response.json();
      setComments([...comments, comment]);
      setNewComment("");
    } catch (err) {
      console.error("Failed to add comment:", err);
    }
  };

  const handleSave = () => {
    if (!card) return;
    dispatch(updateCard({
      id: card.id,
      data: { title, description, priority, working_directory: workingDir, linked_documents: linkedDocs, ai_agent: aiAgent || "" },
    }));
  };

  const handleDelete = () => {
    if (!card) return;
    if (confirm("Are you sure you want to delete this card?")) {
      dispatch(deleteCard(card.id));
      onClose();
    }
  };

  const handleGeneratePlan = async () => {
    try {
      await api.generatePlan(cardId);
    } catch (err) {
      console.error("Failed to generate plan:", err);
    }
  };

  const isAiActive = card?.ai_status && ["planning", "dispatched", "working", "queued"].includes(card.ai_status);

  const handleStopAi = async () => {
    try {
      await api.stopAi(cardId);
    } catch (err) {
      console.error("Failed to stop AI:", err);
    }
  };

  if (!card) return null;

  let aiProgress: any = {};
  try {
    aiProgress = typeof card.ai_progress === 'string' ? JSON.parse(card.ai_progress) : (card.ai_progress || {});
  } catch (e) {
    aiProgress = {};
  }
  const aiCompletedTodos = (aiProgress && aiProgress.completed_todos) || 0;
  const aiTotalTodos = (aiProgress && aiProgress.total_todos) || 0;
  const aiProgressPercent = aiTotalTodos > 0 ? (aiCompletedTodos / aiTotalTodos) * 100 : 0;

  return (
    <Dialog open={open} onClose={() => { handleSave(); onClose(); }} maxWidth={false} PaperProps={{ sx: { width: '80vw', maxWidth: '80vw' } }}>
      <DialogHeader>
        <Box sx={{ display: "flex", alignItems: "center", gap: 2, flex: 1 }}>
          <TextField
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            onBlur={handleSave}
            variant="standard"
            fullWidth
            inputProps={{ style: { fontSize: '1.8rem', fontWeight: 600 } }}
          />
          <Chip label={card.stage} color="primary" size="small" />
        </Box>
        <IconButton onClick={() => { handleSave(); onClose(); }}>
          <CloseIcon />
        </IconButton>
      </DialogHeader>

      <DialogContent dividers>
        {/* Priority */}
        <Section>
          <FormControl fullWidth size="small">
            <InputLabel>Priority</InputLabel>
            <Select value={priority} onChange={(e) => { setPriority(e.target.value); dispatch(updateCard({ id: card.id, data: { priority: e.target.value } })); }} label="Priority">
              <MenuItem value="low">Low</MenuItem>
              <MenuItem value="medium">Medium</MenuItem>
              <MenuItem value="high">High</MenuItem>
            </Select>
          </FormControl>
        </Section>

        {/* AI Agent */}
        {(card.stage === "backlog" || card.stage === "plan" || card.stage === "todo") && (
          <Section>
            <FormControl fullWidth size="small">
              <InputLabel>AI Agent</InputLabel>
              <Select value={aiAgent} onChange={(e) => { setAiAgent(e.target.value); dispatch(updateCard({ id: card.id, data: { ai_agent: e.target.value } })); }} label="AI Agent">
                <MenuItem value="">Auto (default)</MenuItem>
                <MenuItem value="sisyphus">sisyphus</MenuItem>
                <MenuItem value="hephaestus">hephaestus</MenuItem>
                <MenuItem value="atlas">atlas</MenuItem>
                <MenuItem value="bmad-master">bmad-master</MenuItem>
                <MenuItem value="architect">architect</MenuItem>
                <MenuItem value="dev">dev</MenuItem>
                <MenuItem value="pm">pm</MenuItem>
                <MenuItem value="qa">qa</MenuItem>
              </Select>
            </FormControl>
          </Section>
        )}

        {/* Description */}
        <Section>
          <SectionTitle variant="subtitle1">Description</SectionTitle>
          <TextField
            fullWidth
            multiline
            rows={4}
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            onBlur={handleSave}
            placeholder="Add description..."
          />
        </Section>

        {card?.stage === 'plan' && (
          <Section>
            <Box sx={{ display: 'flex', gap: 2, alignItems: 'center' }}>
              <Button
                variant="contained"
                color="primary"
                startIcon={<AutoFixHighIcon />}
                onClick={handleGeneratePlan}
                disabled={!!isAiActive}
              >
                {card.ai_status === 'planning' ? 'Generating...' : 'Generate Plan'}
              </Button>
              {isAiActive && (
                <Button
                  variant="contained"
                  color="error"
                  startIcon={<StopCircleIcon />}
                  onClick={handleStopAi}
                  size="small"
                >
                  Stop AI
                </Button>
              )}
              {card.ai_status === 'planning' && <CircularProgress size={20} />}
            </Box>
          </Section>
        )}

        {/* Subtasks */}
        <Section>
          <SectionTitle variant="subtitle1">
            Subtasks ({subtasks.filter(s => s.completed && !s.id.startsWith("_placeholder_")).length}/{subtasks.filter(s => !s.id.startsWith("_placeholder_")).length})
          </SectionTitle>
          {(() => {
            const real = subtasks.filter(s => !s.id.startsWith("_placeholder_"));
            const done = real.filter(s => s.completed).length;
            return <LinearProgress variant="determinate" value={real.length > 0 ? (done / real.length) * 100 : 0} sx={{ mb: 2 }} />;
          })()}

          {phases.map((phase, phaseIdx) => {
            const realTasks = phase.tasks.filter(t => !t.id.startsWith("_placeholder_"));
            return (
              <Paper key={phase.name} variant="outlined" sx={{ mb: 2, p: 1.5, position: 'relative', overflow: 'hidden' }}>
                <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 1 }}>
                  {editingPhaseName === phase.name ? (
                    <TextField
                      size="small"
                      value={editingPhaseValue}
                      onChange={(e) => setEditingPhaseValue(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") handleRenamephase(phase.name, editingPhaseValue);
                        if (e.key === "Escape") setEditingPhaseName(null);
                      }}
                      onBlur={() => handleRenamephase(phase.name, editingPhaseValue)}
                      autoFocus
                      sx={{ maxWidth: 200 }}
                    />
                  ) : (
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
                      <Typography variant="subtitle2" fontWeight={700} color="primary">
                        Phase {phaseIdx + 1}: {phase.name}
                      </Typography>
                      <IconButton size="small" onClick={() => { setEditingPhaseName(phase.name); setEditingPhaseValue(phase.name); }}>
                        <EditIcon sx={{ fontSize: 14 }} />
                      </IconButton>
                    </Box>
                  )}
                  <Typography variant="caption" color="text.secondary">
                    {realTasks.filter(t => t.completed).length}/{realTasks.length}
                  </Typography>
                </Box>

                <DndContext sensors={subtaskSensors} collisionDetection={closestCenter} onDragEnd={(e) => handleSubtaskDragEnd(e, phase.name)}>
                  <SortableContext items={realTasks.map(t => t.id)} strategy={verticalListSortingStrategy}>
                    <List dense disablePadding>
                      {realTasks.map((subtask, taskIdx) => (
                        <SortableSubtask
                          key={subtask.id}
                          subtask={subtask}
                          taskNumber={taskIdx + 1}
                          editingId={editingSubtaskId}
                          editingTitle={editingSubtaskTitle}
                          onEditStart={(id, title) => { setEditingSubtaskId(id); setEditingSubtaskTitle(title); }}
                          onEditSave={handleEditSubtask}
                          onEditChange={setEditingSubtaskTitle}
                          onEditCancel={() => setEditingSubtaskId(null)}
                          onToggle={handleToggleSubtask}
                          onDelete={handleDeleteSubtask}
                        />
                      ))}
                    </List>
                  </SortableContext>
                </DndContext>

                <Box sx={{ display: "flex", gap: 1, mt: 1 }}>
                  <TextField
                    size="small"
                    fullWidth
                    placeholder={`Add task to ${phase.name}...`}
                    value={newSubtaskByPhase[phase.name] || ""}
                    onChange={(e) => setNewSubtaskByPhase({ ...newSubtaskByPhase, [phase.name]: e.target.value })}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") handleAddSubtask(phase.name, phase.order);
                    }}
                  />
                  <Button variant="outlined" size="small" onClick={() => handleAddSubtask(phase.name, phase.order)}>+</Button>
                </Box>
                {(card.ai_status === "planning" || card.ai_status === "working") && (
                  <MiniLarsonScanner />
                )}
              </Paper>
            );
          })}

          <Box sx={{ display: "flex", gap: 1, mt: 1 }}>
            <TextField
              size="small"
              fullWidth
              placeholder="New phase name..."
              value={newPhaseName}
              onChange={(e) => setNewPhaseName(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") handleAddPhase(); }}
            />
            <Button variant="outlined" size="small" startIcon={<AddIcon />} onClick={handleAddPhase} sx={{ whiteSpace: 'nowrap' }}>
              Add Phase
            </Button>
          </Box>
        </Section>

        {/* Attached Files */}
        <Section>
          <SectionTitle variant="subtitle1">Attached Files</SectionTitle>
          <List dense>
            {files.map((file) => (
              <ListItem
                key={file.id}
                secondaryAction={
                  <IconButton edge="end" onClick={() => handleFileDelete(file.id)}>
                    <DeleteIcon />
                  </IconButton>
                }
              >
                <AttachFileIcon sx={{ mr: 2 }} />
                <ListItemText
                  primary={file.original_filename}
                  secondary={`${(file.file_size / 1024).toFixed(2)} KB - ${new Date(file.uploaded_at).toLocaleDateString()}`}
                  onClick={() => handleFileDownload(file.id)}
                  sx={{ cursor: "pointer" }}
                />
              </ListItem>
            ))}
          </List>
          <Button
            component="label"
            variant="outlined"
            startIcon={<CloudUploadIcon />}
            disabled={uploading}
            fullWidth
          >
            {uploading ? "Uploading..." : "Upload Files"}
            <input
              type="file"
              hidden
              multiple
              onChange={handleFileUpload}
            />
          </Button>
        </Section>

        {/* Linked Documents */}
        <Section>
          <SectionTitle variant="subtitle1">Linked Documents</SectionTitle>
          {linkedDocs && linkedDocs.length > 0 && linkedDocs !== "[]" && (
            <List dense sx={{ mb: 1, bgcolor: 'background.paper', borderRadius: 1, border: '1px solid', borderColor: 'divider' }}>
              {(typeof linkedDocs === 'string' ? linkedDocs.split('\n').filter(Boolean) : []).map((docPath, idx) => (
                <ListItem
                  key={idx}
                  secondaryAction={
                    <IconButton edge="end" size="small" onClick={() => {
                      const paths = linkedDocs.split('\n').filter(Boolean);
                      paths.splice(idx, 1);
                      const updated = paths.join('\n');
                      setLinkedDocs(updated);
                      dispatch(updateCard({ id: card.id, data: { linked_documents: updated } }));
                    }}>
                      <DeleteIcon fontSize="small" />
                    </IconButton>
                  }
                >
                  <AttachFileIcon sx={{ mr: 1, fontSize: 18, color: 'text.secondary' }} />
                  <ListItemText 
                    primary={docPath.split('\\').pop() || docPath.split('/').pop()} 
                    secondary={docPath}
                    primaryTypographyProps={{ variant: 'body2' }}
                    secondaryTypographyProps={{ variant: 'caption', sx: { wordBreak: 'break-all' } }}
                  />
                </ListItem>
              ))}
            </List>
          )}
          <Button
            variant="outlined"
            size="small"
            startIcon={<AttachFileIcon />}
            onClick={async () => {
              try {
                 const res = await fetch(`${API_BASE_URL}/api/pick-files`, { method: 'POST' });
                const data = await res.json();
                if (data.paths && data.paths.length > 0) {
                  const existing = linkedDocs && linkedDocs !== '[]' ? linkedDocs.split('\n').filter(Boolean) : [];
                  const merged = [...existing, ...data.paths];
                  const updated = merged.join('\n');
                  setLinkedDocs(updated);
                  dispatch(updateCard({ id: card.id, data: { linked_documents: updated } }));
                }
              } catch (err) {
                console.error("File picker failed:", err);
              }
            }}
          >
            Choose Files
          </Button>
        </Section>

        {/* Working Directory */}
        <Section>
          <SectionTitle variant="subtitle1">Working Directory</SectionTitle>
          <Box sx={{ display: "flex", alignItems: "center", gap: 2 }}>
            <Button 
              variant="outlined" 
              size="small"
              onClick={async () => {
                try {
                   const res = await fetch(`${API_BASE_URL}/api/pick-directory`, { method: 'POST' });
                  const data = await res.json();
                  if (data.path) {
                    setWorkingDir(data.path);
                    dispatch(updateCard({ id: card.id, data: { working_directory: data.path } }));
                  }
                } catch (err) {
                  console.error("Directory picker failed:", err);
                }
              }}
            >
              Choose Directory
            </Button>
            <Typography variant="body2" color={workingDir === "." ? "text.secondary" : "text.primary"} sx={{ wordBreak: 'break-all' }}>
              {workingDir === "." ? "No directory selected" : workingDir}
            </Typography>
          </Box>
        </Section>

        {/* AI Status */}
        {card.ai_status && card.ai_status !== "idle" && (
          <Section>
            <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 1 }}>
              <SectionTitle variant="subtitle1" sx={{ mb: 0 }}>AI Status</SectionTitle>
              {isAiActive && (
                <Button
                  variant="contained"
                  color="error"
                  startIcon={<StopCircleIcon />}
                  onClick={handleStopAi}
                  size="small"
                >
                  Stop AI
                </Button>
              )}
            </Box>
            <Chip
              label={card.ai_status}
              color={
                card.ai_status === "completed" ? "success" :
                card.ai_status === "cancelled" ? "default" :
                card.ai_status === "failed" ? "error" :
                "primary"
              }
              sx={{ mb: 2 }}
            />
            {aiTotalTodos > 0 && (
              <>
                <Typography variant="body2" gutterBottom>
                  Progress: {aiCompletedTodos}/{aiTotalTodos} tasks
                </Typography>
                <LinearProgress variant="determinate" value={aiProgressPercent} sx={{ mb: 1 }} />
                {aiProgress.current_task && (
                  <Typography variant="caption" color="text.secondary">
                    Current: {aiProgress.current_task}
                  </Typography>
                )}
              </>
            )}
            {card.plan_path && (
              <Typography variant="body2" sx={{ mt: 1 }}>
                Plan: <a href={`file://${card.plan_path}`}>{card.plan_path}</a>
              </Typography>
            )}
          </Section>
        )}

        {card && ['plan', 'todo', 'in_progress'].includes(card.stage) && (
          <Section>
            <SectionTitle variant="subtitle1">AI Agent Logs</SectionTitle>
            <AgentLogViewer cardId={card.id} sessionId={card.ai_session_id} aiStatus={card.ai_status} />
          </Section>
        )}

        <Divider sx={{ my: 3 }} />

        {/* Comments */}
        <Section>
          <SectionTitle variant="subtitle1">Comments ({comments.length})</SectionTitle>
          
          {comments.length > 0 && (
            <List dense sx={{ mb: 2, bgcolor: 'background.paper', borderRadius: 1, border: '1px solid', borderColor: 'divider' }}>
              {comments.map((comment) => (
                <ListItem key={comment.id} sx={{ flexDirection: 'column', alignItems: 'flex-start', position: 'relative' }}>
                  <Box sx={{ display: 'flex', justifyContent: 'space-between', width: '100%', mb: 0.5 }}>
                    <Typography variant="caption" fontWeight={600}>
                      {comment.author}
                    </Typography>
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
                      <Typography variant="caption" color="text.secondary">
                        {new Date(comment.created_at).toLocaleString()}
                      </Typography>
                      {editingCommentId === comment.id ? (
                        <IconButton size="small" onClick={() => handleEditComment(comment.id)}>
                          <CheckIcon fontSize="small" />
                        </IconButton>
                      ) : (
                        <IconButton size="small" onClick={() => {
                          setEditingCommentId(comment.id);
                          setEditingCommentContent(comment.content);
                        }}>
                          <EditIcon fontSize="small" />
                        </IconButton>
                      )}
                      <IconButton size="small" onClick={() => handleDeleteComment(comment.id)}>
                        <DeleteIcon fontSize="small" />
                      </IconButton>
                    </Box>
                  </Box>
                  {editingCommentId === comment.id ? (
                    <TextField
                      size="small"
                      fullWidth
                      multiline
                      rows={2}
                      value={editingCommentContent}
                      onChange={(e) => setEditingCommentContent(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" && !e.shiftKey) {
                          e.preventDefault();
                          handleEditComment(comment.id);
                        }
                        if (e.key === "Escape") setEditingCommentId(null);
                      }}
                      autoFocus
                    />
                  ) : (
                    <Typography variant="body2" sx={{ whiteSpace: 'pre-wrap' }}>
                      {comment.content}
                    </Typography>
                  )}
                </ListItem>
              ))}
            </List>
          )}
          
          {/* Add new comment */}
          <TextField
            size="small"
            fullWidth
            multiline
            rows={2}
            placeholder="Add comment..."
            value={newComment}
            onChange={(e) => setNewComment(e.target.value)}
            onKeyPress={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                handleAddComment();
              }
            }}
          />
          <Button size="small" sx={{ mt: 1 }} onClick={handleAddComment}>
            Add Comment
          </Button>
        </Section>

        <Section>
          <SectionTitle variant="subtitle1">Version History</SectionTitle>
          <Button size="small" onClick={handleToggleVersions} sx={{ mb: 1 }}>
            {showVersions ? "Hide" : "Show"} Version History ({versions.length})
          </Button>
          {showVersions && (
            <List
              dense
              sx={{
                bgcolor: "background.paper",
                borderRadius: 1,
                border: "1px solid",
                borderColor: "divider",
                maxHeight: 200,
                overflow: "auto",
              }}
            >
              {versions.map((version) => {
                let snapshot: CardSnapshot = {};
                try {
                  snapshot = JSON.parse(version.snapshot) as CardSnapshot;
                } catch {
                  snapshot = {};
                }

                return (
                  <ListItem
                    key={version.id}
                    secondaryAction={
                      <Button size="small" onClick={() => handleRestore(version.id)}>
                        Restore
                      </Button>
                    }
                  >
                    <ListItemText
                      primary={`${new Date(version.created_at).toLocaleString()} (${version.changed_by})`}
                      secondary={`Title: ${snapshot.title || ""}, Priority: ${snapshot.priority || ""}`}
                    />
                  </ListItem>
                );
              })}
              {versions.length === 0 && (
                <ListItem>
                  <ListItemText primary="No versions yet" />
                </ListItem>
              )}
            </List>
          )}
        </Section>
      </DialogContent>

      <DialogActions sx={{ justifyContent: "space-between", p: 2 }}>
        <Button startIcon={<DeleteIcon />} color="error" onClick={handleDelete}>
          Delete
        </Button>
        <Button variant="contained" onClick={() => { handleSave(); onClose(); }}>
          Close
        </Button>
      </DialogActions>
    </Dialog>
  );
};
