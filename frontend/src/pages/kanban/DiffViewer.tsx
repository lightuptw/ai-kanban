import React, { useEffect, useMemo, useState } from "react";
import {
  Paper,
  Typography,
  Box,
  Chip,
  List,
  ListItemButton,
  Divider,
  CircularProgress,
} from "@mui/material";
import type { DiffResult, FileDiff } from "../../types/kanban";

interface DiffViewerProps {
  diff: DiffResult | null;
  loading: boolean;
}

const getStatusChipColor = (status: string): "success" | "info" | "error" | "default" => {
  if (status === "added") {
    return "success";
  }
  if (status === "modified") {
    return "info";
  }
  if (status === "deleted") {
    return "error";
  }
  return "default";
};

const getStatusLabel = (status: string): string => {
  if (status === "added") {
    return "A";
  }
  if (status === "modified") {
    return "M";
  }
  if (status === "deleted") {
    return "D";
  }
  if (status === "renamed") {
    return "R";
  }
  return status.slice(0, 1).toUpperCase();
};

const DiffLine: React.FC<{ line: string }> = ({ line }) => {
  const isAddition = line.startsWith("+") && !line.startsWith("+++");
  const isDeletion = line.startsWith("-") && !line.startsWith("---");
  const isHunk = line.startsWith("@@");
  const isOldNewPath = line.startsWith("---") || line.startsWith("+++");

  return (
    <Box
      sx={{
        px: 1,
        py: 0.25,
        fontFamily: "monospace",
        fontSize: "0.78rem",
        whiteSpace: "pre-wrap",
        wordBreak: "break-word",
        backgroundColor: isAddition ? "#e6ffec" : isDeletion ? "#ffebe9" : isHunk ? "#edf2f7" : undefined,
        color: isHunk ? "#344054" : isOldNewPath ? "text.secondary" : undefined,
        fontWeight: isHunk ? 600 : undefined,
        opacity: isOldNewPath ? 0.85 : undefined,
      }}
    >
      {line || " "}
    </Box>
  );
};

export const DiffViewer: React.FC<DiffViewerProps> = ({ diff, loading }) => {
  const [selectedPath, setSelectedPath] = useState<string>("");

  useEffect(() => {
    if (diff?.files?.length) {
      setSelectedPath((prev) => (prev && diff.files.some((file) => file.path === prev) ? prev : diff.files[0].path));
      return;
    }
    setSelectedPath("");
  }, [diff]);

  const selectedFile = useMemo<FileDiff | null>(() => {
    if (!diff?.files?.length) {
      return null;
    }
    return diff.files.find((file) => file.path === selectedPath) || diff.files[0];
  }, [diff, selectedPath]);

  if (loading) {
    return (
      <Paper variant="outlined" sx={{ p: 3, display: "flex", justifyContent: "center" }}>
        <CircularProgress size={24} />
      </Paper>
    );
  }

  if (!diff || diff.files.length === 0) {
    return (
      <Paper variant="outlined" sx={{ p: 3 }}>
        <Typography variant="body2" color="text.secondary">
          No code changes found for this card yet.
        </Typography>
      </Paper>
    );
  }

  const fileLines = (selectedFile?.diff || "")
    .split("\n")
    .filter((line) => !line.startsWith("diff --git"));

  return (
    <Paper variant="outlined" sx={{ overflow: "hidden" }}>
      <Box sx={{ px: 2, py: 1.5, borderBottom: 1, borderColor: "divider", backgroundColor: "grey.50" }}>
        <Typography variant="body2" fontWeight={600}>
          {diff.stats.files_changed} files changed, +{diff.stats.additions} additions, -{diff.stats.deletions} deletions
        </Typography>
      </Box>

      <Box sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", md: "260px 1fr" }, minHeight: 320 }}>
        <Box sx={{ borderRight: { xs: 0, md: 1 }, borderBottom: { xs: 1, md: 0 }, borderColor: "divider", overflowY: "auto" }}>
          <List disablePadding>
            {diff.files.map((file, index) => (
              <React.Fragment key={file.path}>
                <ListItemButton selected={selectedFile?.path === file.path} onClick={() => setSelectedPath(file.path)}>
                  <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between", width: "100%", gap: 1 }}>
                    <Box sx={{ minWidth: 0 }}>
                      <Typography variant="body2" sx={{ fontFamily: "monospace", fontSize: "0.75rem" }} noWrap>
                        {file.path}
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        +{file.additions} / -{file.deletions}
                      </Typography>
                    </Box>
                    <Chip
                      size="small"
                      label={getStatusLabel(file.status)}
                      color={getStatusChipColor(file.status)}
                      sx={{ minWidth: 28, fontWeight: 700 }}
                    />
                  </Box>
                </ListItemButton>
                {index < diff.files.length - 1 && <Divider />}
              </React.Fragment>
            ))}
          </List>
        </Box>

        <Box sx={{ overflow: "auto", maxHeight: 520, backgroundColor: "#fafafa" }}>
          <Box sx={{ px: 2, py: 1, borderBottom: 1, borderColor: "divider", position: "sticky", top: 0, backgroundColor: "background.paper", zIndex: 1 }}>
            <Typography variant="body2" sx={{ fontFamily: "monospace" }}>
              {selectedFile?.path}
            </Typography>
          </Box>
          <Box sx={{ py: 1 }}>
            {fileLines.map((line, index) => (
              <DiffLine key={`${selectedFile?.path || "file"}-${index}`} line={line} />
            ))}
          </Box>
        </Box>
      </Box>
    </Paper>
  );
};
