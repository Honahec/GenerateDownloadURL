export interface LoginResponse {
  token: string;
  expires_in: number;
}

export interface CreateLinkRequest {
  bucket?: string;
  object_key: string;
  expires_in_seconds?: number;
  max_downloads?: number;
  download_filename?: string;
}

export interface CreateLinkResponse {
  id: string;
  url: string;
  expires_at: string;
  max_downloads?: number;
}

export interface DownloadLinkResponse {
  id: string;
  object_key: string;
  bucket?: string;
  expires_at: string;
  max_downloads?: number;
  downloads_served: number;
  created_at: string;
  download_filename?: string;
  is_expired: boolean;
  download_url: string;
}

export interface ListLinksResponse {
  links: DownloadLinkResponse[];
  total: number;
}
