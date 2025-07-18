type GitInfo = {
  lastModified: string;
  author: string;
  hash: string;
}

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
}

const gitCache = new Map<string, GitInfo>();
const GITHUB_API_BASE = 'https://api.github.com';
const REPO_OWNER = 'karol-broda';
const REPO_NAME = 'funnel';

async function fetchWithRetry(url: string, retries = 3): Promise<Response> {
  for (let i = 0; i < retries; i++) {
    try {
      const response = await fetch(url, {
        headers: {
          'Accept': 'application/vnd.github.v3+json',
          'User-Agent': 'funnel-docs',
        },
        next: { revalidate: process.env.NODE_ENV === 'development' ? 300 : 3600 }
      });

      if (response.ok) {
        return response;
      }
      
      if (response.status === 403 && i < retries - 1) {
        await new Promise(resolve => setTimeout(resolve, 1000 * (i + 1)));
        continue;
      }
      
      throw new Error(`GitHub API error: ${response.status}`);
    } catch (error) {
      if (i === retries - 1) throw error;
      await new Promise(resolve => setTimeout(resolve, 1000 * (i + 1)));
    }
  }
  
  throw new Error('Max retries exceeded');
}

export async function getGitInfo(filePath: string): Promise<GitInfo | null> {
  if (gitCache.has(filePath)) {
    return gitCache.get(filePath)!;
  }

  try {
    const apiPath = filePath.startsWith('/') ? filePath.slice(1) : filePath;
    
    const url = `${GITHUB_API_BASE}/repos/${REPO_OWNER}/${REPO_NAME}/commits?path=${encodeURIComponent(apiPath)}&per_page=1`;
    
    const response = await fetchWithRetry(url);
    const commits: GitHubCommit[] = await response.json();

    if (!commits || commits.length === 0) {
      return null;
    }

    const latestCommit = commits[0];
    
    const gitInfo: GitInfo = {
      lastModified: latestCommit.commit.author.date,
      author: latestCommit.author?.login || latestCommit.commit.author.name,
      hash: latestCommit.sha
    };

    gitCache.set(filePath, gitInfo);
    
    return gitInfo;
  } catch (error) {
    console.warn(`Failed to fetch git info for ${filePath}:`, error);
    return null;
  }
}

export async function getFileLastModified(filePath: string): Promise<string> {
  const gitInfo = await getGitInfo(filePath);
  return gitInfo?.lastModified || new Date().toISOString();
}

export async function getFileAuthor(filePath: string): Promise<string> {
  const gitInfo = await getGitInfo(filePath);
  return gitInfo?.author || 'Unknown';
}

export async function getLastModifiedFromUrl(url: string): Promise<string> {
  const basePath = url === '/docs' ? 'docs/content/docs' : url.replace('/docs', 'docs/content/docs');
  
  const paths = [
    basePath + '.mdx',
    basePath + '/index.mdx'
  ];
  
  for (const path of paths) {
    const result = await getFileLastModified(path);
    if (result !== new Date().toISOString()) {
      return result;
    }
  }
  
  return new Date().toISOString();
}

export async function getAuthorFromUrl(url: string): Promise<string> {
  const basePath = url === '/docs' ? 'docs/content/docs' : url.replace('/docs', 'docs/content/docs');
  
  const paths = [
    basePath + '.mdx',
    basePath + '/index.mdx'
  ];
  
  for (const path of paths) {
    const result = await getFileAuthor(path);
    if (result !== 'Unknown') {
      return result;
    }
  }
  
  return 'Unknown';
}

export async function getFileCreated(filePath: string): Promise<string> {
  try {
    const apiPath = filePath.startsWith('/') ? filePath.slice(1) : filePath;
    
    const url = `${GITHUB_API_BASE}/repos/${REPO_OWNER}/${REPO_NAME}/commits?path=${encodeURIComponent(apiPath)}&per_page=100`;
    
    const response = await fetchWithRetry(url);
    const commits: GitHubCommit[] = await response.json();

    if (!commits || commits.length === 0) {
      return new Date().toISOString();
    }

    const oldestCommit = commits[commits.length - 1];
    return oldestCommit.commit.author.date;
  } catch (error) {
    console.warn(`Failed to fetch creation date for ${filePath}:`, error);
    return new Date().toISOString();
  }
} 

export async function getCreatedFromUrl(url: string): Promise<string> {
  const basePath = url === '/docs' ? 'docs/content/docs' : url.replace('/docs', 'docs/content/docs');
  
  const paths = [
    basePath + '.mdx',
    basePath + '/index.mdx'
  ];
  
  for (const path of paths) {
    const result = await getFileCreated(path);
    if (result !== new Date().toISOString()) {
      return result;
    }
  }
  
  return new Date().toISOString();
} 