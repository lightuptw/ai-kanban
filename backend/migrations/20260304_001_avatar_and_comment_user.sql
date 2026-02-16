-- Add avatar_content_type to users table (for serving correct MIME type)
ALTER TABLE users ADD COLUMN avatar_content_type TEXT;

-- Add user_id to comments table (for resolving comment author avatars)
ALTER TABLE comments ADD COLUMN user_id TEXT;
