import React, { useState, useCallback, useRef } from "react";
import Cropper from "react-easy-crop";
import type { Area } from "react-easy-crop";
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  Box,
  CircularProgress,
  Alert,
  Typography,
} from "@mui/material";
import type { AuthUser } from "../services/auth";
import { uploadAvatar, deleteAvatar, getUser } from "../services/auth";

const MAX_FILE_SIZE = 2 * 1024 * 1024;
const ACCEPTED_TYPES = "image/jpeg,image/png,image/webp";

function getCroppedBlob(
  imageSrc: string,
  crop: Area
): Promise<Blob> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.crossOrigin = "anonymous";
    image.onload = () => {
      const canvas = document.createElement("canvas");
      canvas.width = crop.width;
      canvas.height = crop.height;
      const ctx = canvas.getContext("2d");
      if (!ctx) {
        reject(new Error("Could not get canvas context"));
        return;
      }
      ctx.drawImage(
        image,
        crop.x,
        crop.y,
        crop.width,
        crop.height,
        0,
        0,
        crop.width,
        crop.height
      );
      canvas.toBlob(
        (blob) => {
          if (blob) resolve(blob);
          else reject(new Error("Canvas toBlob failed"));
        },
        "image/jpeg",
        0.9
      );
    };
    image.onerror = () => reject(new Error("Failed to load image"));
    image.src = imageSrc;
  });
}

interface AvatarUploadDialogProps {
  open: boolean;
  onClose: () => void;
  onSuccess: (user: AuthUser) => void;
}

const AvatarUploadDialog: React.FC<AvatarUploadDialogProps> = ({
  open,
  onClose,
  onSuccess,
}) => {
  const [imageSrc, setImageSrc] = useState<string | null>(null);
  const [crop, setCrop] = useState({ x: 0, y: 0 });
  const [zoom, setZoom] = useState(1);
  const [croppedArea, setCroppedArea] = useState<Area | null>(null);
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const currentUser = getUser();

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    if (file.size > MAX_FILE_SIZE) {
      setError("File too large. Maximum size is 2MB.");
      return;
    }

    const reader = new FileReader();
    reader.onload = () => {
      setImageSrc(reader.result as string);
      setError(null);
      setCrop({ x: 0, y: 0 });
      setZoom(1);
    };
    reader.readAsDataURL(file);
  };

  const onCropComplete = useCallback((_: Area, croppedAreaPixels: Area) => {
    setCroppedArea(croppedAreaPixels);
  }, []);

  const handleUpload = async () => {
    if (!imageSrc || !croppedArea) return;

    setUploading(true);
    setError(null);
    try {
      const blob = await getCroppedBlob(imageSrc, croppedArea);
      const file = new File([blob], "avatar.jpg", { type: "image/jpeg" });
      const user = await uploadAvatar(file);
      onSuccess(user);
      handleReset();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Upload failed");
    } finally {
      setUploading(false);
    }
  };

  const handleRemove = async () => {
    setUploading(true);
    setError(null);
    try {
      await deleteAvatar();
      const updatedUser = getUser();
      if (updatedUser) onSuccess(updatedUser);
      handleReset();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to remove avatar");
    } finally {
      setUploading(false);
    }
  };

  const handleReset = () => {
    setImageSrc(null);
    setCrop({ x: 0, y: 0 });
    setZoom(1);
    setCroppedArea(null);
    setError(null);
    if (fileInputRef.current) fileInputRef.current.value = "";
  };

  const handleClose = () => {
    handleReset();
    onClose();
  };

  return (
    <Dialog open={open} onClose={handleClose} maxWidth="sm" fullWidth>
      <DialogTitle>Change Avatar</DialogTitle>
      <DialogContent>
        {error && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {error}
          </Alert>
        )}

        {!imageSrc ? (
          <Box sx={{ textAlign: "center", py: 4 }}>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              Select an image (JPEG, PNG, or WebP, max 2MB)
            </Typography>
            <Button variant="outlined" component="label">
              Choose Image
              <input
                ref={fileInputRef}
                type="file"
                hidden
                accept={ACCEPTED_TYPES}
                onChange={handleFileChange}
              />
            </Button>
          </Box>
        ) : (
          <Box sx={{ position: "relative", width: "100%", height: 400 }}>
            <Cropper
              image={imageSrc}
              crop={crop}
              zoom={zoom}
              aspect={1}
              onCropChange={setCrop}
              onZoomChange={setZoom}
              onCropComplete={onCropComplete}
            />
          </Box>
        )}
      </DialogContent>
      <DialogActions sx={{ justifyContent: "space-between", px: 3, pb: 2 }}>
        <Box>
          {currentUser?.has_avatar && (
            <Button
              color="error"
              onClick={handleRemove}
              disabled={uploading}
            >
              Remove Avatar
            </Button>
          )}
        </Box>
        <Box sx={{ display: "flex", gap: 1 }}>
          <Button onClick={handleClose} disabled={uploading}>
            Cancel
          </Button>
          {imageSrc && (
            <Button
              variant="contained"
              onClick={handleUpload}
              disabled={uploading || !croppedArea}
            >
              {uploading ? <CircularProgress size={20} /> : "Save"}
            </Button>
          )}
        </Box>
      </DialogActions>
    </Dialog>
  );
};

export default AvatarUploadDialog;
