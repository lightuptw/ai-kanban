import React, { useEffect, useMemo, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Chip,
  LinearProgress,
  List,
  ListItemButton,
  Paper,
  Typography,
} from "@mui/material";
import {
  CheckCircleOutline as CheckCircleOutlineIcon,
  RadioButtonUnchecked as RadioButtonUncheckedIcon,
} from "@mui/icons-material";
import { api } from "../../services/api";
import type { ConflictDetail, ConflictFile, FileResolution } from "../../types/kanban";
import { ConflictFileViewer } from "./ConflictFileViewer";
import { ManualResolveEditor } from "./ManualResolveEditor";

interface ConflictResolutionPanelProps {
  cardId: string;
  conflictDetail: ConflictDetail;
  onMergeComplete: () => void;
  onMergeAbort: () => void;
}

export const ConflictResolutionPanel: React.FC<ConflictResolutionPanelProps> = ({
  cardId,
  conflictDetail,
  onMergeComplete,
  onMergeAbort,
}) => {
  const [currentDetail, setCurrentDetail] = useState<ConflictDetail>(conflictDetail);
  const [selectedPath, setSelectedPath] = useState<string>(conflictDetail.files[0]?.path || "");
  const [resolutionMap, setResolutionMap] = useState<Record<string, FileResolution>>({});
  const [initialPaths, setInitialPaths] = useState<string[]>(
    conflictDetail.files.map((file) => file.path)
  );
  const [manualResolveOpen, setManualResolveOpen] = useState(false);
  const [resolving, setResolving] = useState(false);
  const [completingMerge, setCompletingMerge] = useState(false);
  const [abortingMerge, setAbortingMerge] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    setCurrentDetail(conflictDetail);
    setSelectedPath(conflictDetail.files[0]?.path || "");
    setResolutionMap({});
    setInitialPaths(conflictDetail.files.map((file) => file.path));
    setManualResolveOpen(false);
    setErrorMessage(null);
  }, [conflictDetail]);

  useEffect(() => {
    if (!currentDetail.files.length) {
      setSelectedPath("");
      return;
    }
    if (!selectedPath || !currentDetail.files.some((file) => file.path === selectedPath)) {
      setSelectedPath(currentDetail.files[0].path);
    }
  }, [currentDetail.files, selectedPath]);

  const selectedFile = useMemo<ConflictFile | null>(() => {
    if (!currentDetail.files.length) {
      return null;
    }
    return currentDetail.files.find((file) => file.path === selectedPath) || currentDetail.files[0];
  }, [currentDetail.files, selectedPath]);

  const resolvedCount = useMemo(() => {
    const unresolved = new Set(currentDetail.files.map((file) => file.path));
    return initialPaths.filter((path) => !unresolved.has(path) || Boolean(resolutionMap[path])).length;
  }, [currentDetail.files, initialPaths, resolutionMap]);

  const totalCount = initialPaths.length;
  const allResolved = totalCount > 0 && resolvedCount === totalCount;
  const progressValue = totalCount > 0 ? (resolvedCount / totalCount) * 100 : 0;

  const resolveSelectedFile = async (choice: string, manualContent?: string) => {
    if (!selectedFile) {
      return;
    }

    setResolving(true);
    setErrorMessage(null);

    const resolution: FileResolution = {
      file_path: selectedFile.path,
      choice,
      ...(manualContent !== undefined ? { manual_content: manualContent } : {}),
    };

    try {
      const updatedDetail = await api.resolveConflicts(cardId, [resolution]);
      setCurrentDetail(updatedDetail);
      setResolutionMap((prev) => ({ ...prev, [selectedFile.path]: resolution }));
      setManualResolveOpen(false);
    } catch {
      setErrorMessage("Failed to apply resolution.");
    } finally {
      setResolving(false);
    }
  };

  const handleCompleteMerge = async () => {
    setCompletingMerge(true);
    setErrorMessage(null);
    try {
      const result = await api.completeMerge(cardId);
      if (!result.success) {
        setErrorMessage(result.message || "Unable to complete merge.");
        return;
      }
      onMergeComplete();
    } catch {
      setErrorMessage("Failed to complete merge.");
    } finally {
      setCompletingMerge(false);
    }
  };

  const handleAbortMerge = async () => {
    setAbortingMerge(true);
    setErrorMessage(null);
    try {
      await api.abortMerge(cardId);
      onMergeAbort();
    } catch {
      setErrorMessage("Failed to abort merge.");
    } finally {
      setAbortingMerge(false);
    }
  };

  return (
    <Paper variant="outlined" sx={{ overflow: "hidden" }}>
      <Box sx={{ px: 2, py: 1.5, borderBottom: 1, borderColor: "divider", backgroundColor: "grey.50" }}>
        <Typography variant="body2" fontWeight={600}>
          {resolvedCount} of {totalCount} resolved
        </Typography>
        <LinearProgress variant="determinate" value={progressValue} sx={{ mt: 1 }} />
        {!currentDetail.merge_in_progress && (
          <Typography variant="caption" color="text.secondary" sx={{ display: "block", mt: 1 }}>
            Merge is currently paused. Resolve all files and complete merge to finish.
          </Typography>
        )}
      </Box>

      {errorMessage && (
        <Alert severity="error" sx={{ m: 2, mb: 0 }}>
          {errorMessage}
        </Alert>
      )}

      <Box sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", md: "260px 1fr" }, minHeight: 440 }}>
        <Box sx={{ borderRight: { xs: 0, md: 1 }, borderBottom: { xs: 1, md: 0 }, borderColor: "divider", overflowY: "auto" }}>
          <List disablePadding>
            {currentDetail.files.map((file) => {
              const isResolved = Boolean(resolutionMap[file.path]);
              return (
                <ListItemButton
                  key={file.path}
                  selected={selectedFile?.path === file.path}
                  onClick={() => setSelectedPath(file.path)}
                >
                  <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between", width: "100%", gap: 1 }}>
                    <Box sx={{ minWidth: 0 }}>
                      <Typography variant="body2" sx={{ fontFamily: "monospace", fontSize: "0.74rem" }} noWrap>
                        {file.path}
                      </Typography>
                      <Chip
                        size="small"
                        label={file.conflict_type}
                        variant="outlined"
                        sx={{ mt: 0.5, maxWidth: "100%", height: 20, fontSize: "0.68rem" }}
                      />
                    </Box>
                    {isResolved ? (
                      <CheckCircleOutlineIcon color="success" fontSize="small" />
                    ) : (
                      <RadioButtonUncheckedIcon color="disabled" fontSize="small" />
                    )}
                  </Box>
                </ListItemButton>
              );
            })}
          </List>
        </Box>

        <Box sx={{ p: 2, display: "flex", flexDirection: "column", gap: 2 }}>
          {selectedFile ? (
            <>
              <ConflictFileViewer file={selectedFile} />
              <Box sx={{ display: "flex", gap: 1, flexWrap: "wrap" }}>
                <Button
                  variant="contained"
                  onClick={() => resolveSelectedFile("ours")}
                  disabled={resolving || completingMerge || abortingMerge}
                >
                  Accept Ours
                </Button>
                <Button
                  variant="contained"
                  color="success"
                  onClick={() => resolveSelectedFile("theirs")}
                  disabled={resolving || completingMerge || abortingMerge}
                >
                  Accept Theirs
                </Button>
                <Button
                  variant="outlined"
                  onClick={() => setManualResolveOpen(true)}
                  disabled={selectedFile.is_binary || resolving || completingMerge || abortingMerge}
                >
                  Edit Manually
                </Button>
              </Box>
            </>
          ) : (
            <Paper
              variant="outlined"
              sx={{ p: 4, minHeight: 360, display: "flex", alignItems: "center", justifyContent: "center" }}
            >
              <Typography variant="body2" color="text.secondary">
                No unresolved files remain.
              </Typography>
            </Paper>
          )}
        </Box>
      </Box>

      <Box sx={{ p: 2, borderTop: 1, borderColor: "divider", display: "flex", justifyContent: "space-between", gap: 1 }}>
        <Button
          variant="contained"
          color="success"
          onClick={handleCompleteMerge}
          disabled={!allResolved || completingMerge || abortingMerge || resolving}
        >
          {completingMerge ? "Completing..." : "Complete Merge"}
        </Button>
        <Button
          variant="outlined"
          color="error"
          onClick={handleAbortMerge}
          disabled={abortingMerge || completingMerge}
        >
          {abortingMerge ? "Aborting..." : "Abort Merge"}
        </Button>
      </Box>

      {selectedFile && (
        <ManualResolveEditor
          open={manualResolveOpen}
          file={selectedFile}
          onResolve={(content) => resolveSelectedFile("manual", content)}
          onCancel={() => setManualResolveOpen(false)}
        />
      )}
    </Paper>
  );
};
