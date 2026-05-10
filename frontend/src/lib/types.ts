export interface UserStats {
  id: number;
  account_id: number;
  username: string;
  role: 'user' | 'verified' | 'moderator' | 'admin';
  upload_count: number;
  accepted_upload_count: number;
  pending_upload_count: number;
  level_count: number;
  accepted_level_count: number;
  active_thumbnail_count: number;
}

export interface StatsResponse {
  storage: number;
  thumbnails: number;
  users_per_month: number;
  users_total: number;
  uploads_total: number;
  pending_uploads_total: number;
  accepted_uploads_total: number;
  total_levels: number;
  current_pending_uploads: number;
}

export interface UserHistoryPoint {
  period: string;
  upload_count: number;
  accepted_upload_count: number;
  pending_upload_count: number;
  level_count: number;
  accepted_level_count: number;
}

export interface StatsHistoryPoint {
  captured_at: string;
  storage_bytes: number;
  thumbnails_count: number;
  users_per_month: number | null;
  users_total: number;
  uploads_total: number;
  pending_uploads_total: number;
  accepted_uploads_total: number;
}

export interface ServerSettings {
  pause_submissions: boolean;
  min_supported_client: string;
}

export interface PendingItem {
  id: number;
  user_id: number;
  username: string;
  level_id: number;
  accepted: boolean;
  replacement: boolean;
  upload_time: string;
}

export interface PendingResponse {
  uploads: PendingItem[];
  page: number;
  per_page: number;
  total: number;
}
