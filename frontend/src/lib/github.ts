import { apiFetch } from "./api";

export interface GitHubRepo {
  full_name: string;
  private: boolean;
  default_branch: string;
  html_url: string;
}

export interface GitHubReposResponse {
  github_login: string;
  repos: GitHubRepo[];
}

/** 404 means "no GitHub account connected yet" — a real state, not an
 * error, so it's translated to `null` here rather than left to reject the
 * query and get treated the same as an actual failure. Shared by
 * Dashboard's connect panel and AgentDetail's deploy-from-repo form —
 * both just need to know "connected, and with what repos" or not. */
export async function fetchGitHubConnection(): Promise<GitHubReposResponse | null> {
  try {
    return await apiFetch<GitHubReposResponse>("/github/repos");
  } catch (err) {
    if (err instanceof Error && /\s404\s/.test(err.message)) {
      return null;
    }
    throw err;
  }
}

/** `full_name` (e.g. "owner/repo") to the plain clone URL the control
 * plane expects in a deploy-from-repo request — deliberately built from
 * `full_name` rather than the API's `html_url`, since a `.git` suffix is
 * what makes Docker's build API reliably recognize this as a git context
 * rather than an HTTP context. */
export function repoUrlFor(fullName: string): string {
  return `https://github.com/${fullName}.git`;
}
