import React, { useEffect, useMemo, useState } from "react";
import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  TextField,
} from "@mui/material";
import type { ConflictFile } from "../../types/kanban";

interface ManualResolveEditorProps {
  open: boolean;
  file: ConflictFile;
  onResolve: (content: string) => void;
  onCancel: () => void;
}

export const ManualResolveEditor: React.FC<ManualResolveEditorProps> = ({ open, file, onResolve, onCancel }) => {
  const initialContent = useMemo(() => file.theirs_content ?? file.ours_content ?? "", [file]);
  const [content, setContent] = useState(initialContent);

  useEffect(() => {
    if (open) {
      setContent(initialContent);
    }
  }, [open, initialContent]);

  const filename = file.path.split("/").at(-1) || file.path;

  return (
    <Dialog open={open} onClose={onCancel} maxWidth="lg" fullWidth>
      <DialogTitle>{`Manually Resolve: ${filename}`}</DialogTitle>
      <DialogContent dividers>
        <TextField
          fullWidth
          multiline
          minRows={20}
          value={content}
          onChange={(event) => setContent(event.target.value)}
          sx={{
            "& .MuiInputBase-input": {
              fontFamily: "monospace",
              fontSize: "0.85rem",
              minHeight: 400,
            },
          }}
        />
      </DialogContent>
      <DialogActions>
        <Button onClick={onCancel}>Cancel</Button>
        <Button variant="contained" onClick={() => onResolve(content)}>
          Apply Resolution
        </Button>
      </DialogActions>
    </Dialog>
  );
};
