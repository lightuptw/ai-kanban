import React from "react";
import { Box, Paper, Typography } from "@mui/material";
import type { ConflictFile } from "../../types/kanban";

interface ConflictFileViewerProps {
  file: ConflictFile;
}

const ContentColumn: React.FC<{ title: string; color: string; content: string | null }> = ({ title, color, content }) => {
  const lines = (content ?? "").split("\n");

  return (
    <Paper
      variant="outlined"
      sx={{
        display: "flex",
        flexDirection: "column",
        minHeight: 360,
        overflow: "hidden",
      }}
    >
      <Box sx={{ px: 2, py: 1, borderBottom: 1, borderColor: "divider", backgroundColor: color }}>
        <Typography variant="body2" fontWeight={700} color="common.white">
          {title}
        </Typography>
      </Box>
      <Box sx={{ flex: 1, overflow: "auto", backgroundColor: "#fbfbfc" }}>
        {content === null ? (
          <Box sx={{ p: 3 }}>
            <Typography variant="body2" color="text.secondary">
              File does not exist on this side.
            </Typography>
          </Box>
        ) : (
          lines.map((line, index) => (
            <Box
              key={`${title}-${index}`}
              sx={{
                display: "grid",
                gridTemplateColumns: "52px 1fr",
                borderBottom: 1,
                borderColor: "divider",
                minHeight: 24,
              }}
            >
              <Box
                sx={{
                  px: 1,
                  py: 0.25,
                  borderRight: 1,
                  borderColor: "divider",
                  fontFamily: "monospace",
                  fontSize: "0.75rem",
                  color: "text.secondary",
                  textAlign: "right",
                  userSelect: "none",
                  backgroundColor: "#f4f6f8",
                }}
              >
                {index + 1}
              </Box>
              <Box
                sx={{
                  px: 1,
                  py: 0.25,
                  fontFamily: "monospace",
                  fontSize: "0.78rem",
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-word",
                }}
              >
                {line || " "}
              </Box>
            </Box>
          ))
        )}
      </Box>
    </Paper>
  );
};

export const ConflictFileViewer: React.FC<ConflictFileViewerProps> = ({ file }) => {
  const isDeletedConflict = file.conflict_type.toLowerCase().includes("delete") || file.ours_content === null || file.theirs_content === null;

  if (file.is_binary) {
    return (
      <Paper variant="outlined" sx={{ p: 4, minHeight: 360, display: "flex", alignItems: "center", justifyContent: "center" }}>
        <Typography variant="body1" color="text.secondary" sx={{ textAlign: "center" }}>
          Binary file - choose Ours or Theirs
        </Typography>
      </Paper>
    );
  }

  if (isDeletedConflict) {
    const oursDeleted = file.ours_content === null;
    return (
      <Paper variant="outlined" sx={{ p: 4, minHeight: 360, display: "flex", alignItems: "center", justifyContent: "center" }}>
        <Box sx={{ textAlign: "center" }}>
          <Typography variant="body1" fontWeight={600} gutterBottom>
            Delete conflict detected
          </Typography>
          <Typography variant="body2" color="text.secondary">
            {oursDeleted ? "Ours deleted this file while Theirs modified or kept it." : "Theirs deleted this file while Ours modified or kept it."}
          </Typography>
        </Box>
      </Paper>
    );
  }

  return (
    <Box sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", md: "1fr 1fr" }, gap: 2 }}>
      <ContentColumn title="Ours (Current)" color="#2b6cb0" content={file.ours_content} />
      <ContentColumn title="Theirs (Incoming)" color="#2f855a" content={file.theirs_content} />
    </Box>
  );
};
