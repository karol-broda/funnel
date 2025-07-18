import { unstable_cache } from "next/cache";

type GitInfo = {
  lastModified: string;
  author: string;
  hash: string;
};

type GitHubCommit = {
  sha: string;
  commit: {
    author: {
      name: string;
      date: string;
    };
    message: string;
  };
  author: {
    login: string;
    avatar_url: string;
  } | null;
};

const gitCache = new Map<string, GitInfo>();
const GITHUB_API_BASE = "https://api.github.com";

function getGitHubToken(): string | null {
  return process.env.GITHUB_TOKEN || null;
}

function getRepoInfo(): { owner: string; name: string } | null {
  const vercelOwner =
    process.env.VERCEL_GIT_REPO_OWNER ||
    process.env.NEXT_PUBLIC_VERCEL_GIT_REPO_OWNER;
  const vercelSlug =
    process.env.VERCEL_GIT_REPO_SLUG ||
    process.env.NEXT_PUBLIC_VERCEL_GIT_REPO_SLUG;

  if (vercelOwner && vercelSlug) {
    return { owner: vercelOwner, name: vercelSlug };
  }

  const githubRepo = process.env.GITHUB_REPOSITORY;
  if (githubRepo && githubRepo.includes("/")) {
    const [owner, name] = githubRepo.split("/");
    return { owner, name };
  }

  return { owner: "karol-broda", name: "funnel" };
}

async function fetchWithRetry(
  url: string,
  retries = 2
): Promise<Response | null> {
  for (let i = 0; i < retries; i++) {
    try {
      const response = await fetch(url, {
        headers: {
          Accept: "application/vnd.github.v3+json",
          "User-Agent": "tunneling-docs",
          ...(getGitHubToken() && {
            Authorization: `Bearer ${getGitHubToken()}`,
          }),
        },
      });

      if (response.ok) {
        return response;
      }

      // silently handle expected GitHub API issues
      if (response.status === 403 || response.status === 404) {
        return null;
      }

      if (i < retries - 1) {
        await new Promise((resolve) => setTimeout(resolve, 1000 * (i + 1)));
        continue;
      }

      return null;
    } catch {
      if (i === retries - 1) {
        return null;
      }
      await new Promise((resolve) => setTimeout(resolve, 1000 * (i + 1)));
    }
  }

  return null;
}

export const getGitInfo = unstable_cache(
  async (filePath: string): Promise<GitInfo | null> => {
    if (gitCache.has(filePath)) {
      return gitCache.get(filePath)!;
    }

    const repoInfo = getRepoInfo();
    if (!repoInfo) {
      return null;
    }

    try {
      const apiPath = filePath.startsWith("/") ? filePath.slice(1) : filePath;

      const url = `${GITHUB_API_BASE}/repos/${repoInfo.owner}/${
        repoInfo.name
      }/commits?path=${encodeURIComponent(apiPath)}&per_page=1`;

      const response = await fetchWithRetry(url);
      if (!response) {
        return null;
      }

      const commits: GitHubCommit[] = await response.json();

      if (!commits || commits.length === 0) {
        return null;
      }

      const latestCommit = commits[0];

      const gitInfo: GitInfo = {
        lastModified: latestCommit.commit.author.date,
        author: latestCommit.author?.login || latestCommit.commit.author.name,
        hash: latestCommit.sha,
      };

      gitCache.set(filePath, gitInfo);

      return gitInfo;
    } catch {
      return null;
    }
  },
  ["git-info"],
  {
    tags: ["git-info"],
    revalidate: false,
  }
);

export const getFileLastModified = unstable_cache(
  async (filePath: string): Promise<string> => {
    const gitInfo = await getGitInfo(filePath);
    return gitInfo?.lastModified || new Date().toISOString();
  },
  ["file-last-modified"],
  {
    tags: ["git-info"],
    revalidate: false,
  }
);

export const getFileAuthor = unstable_cache(
  async (filePath: string): Promise<string> => {
    const gitInfo = await getGitInfo(filePath);
    return gitInfo?.author || "Unknown";
  },
  ["file-author"],
  {
    tags: ["git-info"],
    revalidate: false,
  }
);

export const getLastModifiedFromUrl = unstable_cache(
  async (url: string): Promise<string> => {
    const basePath =
      url === "/docs"
        ? "docs/content/docs"
        : url.replace("/docs", "docs/content/docs");

    const paths = [basePath + ".mdx", basePath + "/index.mdx"];

    for (const path of paths) {
      const result = await getFileLastModified(path);
      if (result !== new Date().toISOString()) {
        return result;
      }
    }

    return new Date().toISOString();
  },
  ["last-modified"],
  {
    tags: ["git-info"],
    revalidate: false, // never revalidate after build
  }
);

export const getAuthorFromUrl = unstable_cache(
  async (url: string): Promise<string> => {
    const basePath =
      url === "/docs"
        ? "docs/content/docs"
        : url.replace("/docs", "docs/content/docs");

    const paths = [basePath + ".mdx", basePath + "/index.mdx"];

    for (const path of paths) {
      const result = await getFileAuthor(path);
      if (result !== "Unknown") {
        return result;
      }
    }

    return "Unknown";
  },
  ["author"],
  {
    tags: ["git-info"],
    revalidate: false, // never revalidate after build
  }
);

export const getFileCreated = unstable_cache(
  async (filePath: string): Promise<string> => {
    const repoInfo = getRepoInfo();
    if (!repoInfo) {
      return new Date().toISOString();
    }

    try {
      const apiPath = filePath.startsWith("/") ? filePath.slice(1) : filePath;

      const url = `${GITHUB_API_BASE}/repos/${repoInfo.owner}/${
        repoInfo.name
      }/commits?path=${encodeURIComponent(apiPath)}&per_page=100`;

      const response = await fetchWithRetry(url);
      if (!response) {
        return new Date().toISOString();
      }

      const commits: GitHubCommit[] = await response.json();

      if (!commits || commits.length === 0) {
        return new Date().toISOString();
      }

      const oldestCommit = commits[commits.length - 1];
      return oldestCommit.commit.author.date;
    } catch {
      return new Date().toISOString();
    }
  },
  ["file-created"],
  {
    tags: ["git-info"],
    revalidate: false,
  }
);

export const getCreatedFromUrl = unstable_cache(
  async (url: string): Promise<string> => {
    const basePath =
      url === "/docs"
        ? "docs/content/docs"
        : url.replace("/docs", "docs/content/docs");

    const paths = [basePath + ".mdx", basePath + "/index.mdx"];

    for (const path of paths) {
      const result = await getFileCreated(path);
      if (result !== new Date().toISOString()) {
        return result;
      }
    }

    return new Date().toISOString();
  },
  ["created"],
  {
    tags: ["git-info"],
    revalidate: false, // never revalidate after build
  }
);
