"use client";

import { CalendarIcon, UserIcon } from "@phosphor-icons/react";
import Link from "fumadocs-core/link";

interface LastModifiedProps {
  date: string;
  author?: string;
}

export default function LastModified({ date, author }: LastModifiedProps) {
  const formattedDate = new Date(date).toLocaleDateString("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
  });

  return (
    <div className="mt-8 pt-6 border-t border-border">
      <div className="flex items-center gap-4 text-sm text-muted-foreground">
        <div className="flex items-center gap-1">
          <CalendarIcon className="h-4 w-4" />
          <span>Last updated: {formattedDate}</span>
        </div>
        {author && author !== "Unknown" && (
          <Link
            className="flex items-center gap-1"
            href={`https://github.com/${author}`}
            target="_blank"
            rel="noopener noreferrer"
          >
            <UserIcon className="h-4 w-4" />
            <span>by {author}</span>
          </Link>
        )}
      </div>
    </div>
  );
}
