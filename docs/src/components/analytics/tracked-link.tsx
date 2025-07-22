"use client";

import Link from "next/link";
import { useDocumentationTracking } from "@/hooks/use-analytics";
import { ReactNode, MouseEventHandler } from "react";

interface TrackedLinkProps {
  href: string;
  children: ReactNode;
  className?: string;
  target?: string;
  rel?: string;
  context: string;
  onClick?: MouseEventHandler<HTMLAnchorElement>;
}

export function TrackedLink({
  href,
  children,
  className,
  target,
  rel,
  context,
  onClick,
  ...props
}: TrackedLinkProps) {
  const { trackExternalLink } = useDocumentationTracking();

  const handleClick: MouseEventHandler<HTMLAnchorElement> = (event) => {
    if (href.startsWith("http")) {
      trackExternalLink(href, context);
    }

    if (onClick) {
      onClick(event);
    }
  };

  return (
    <Link
      href={href}
      className={className}
      target={target}
      rel={rel}
      onClick={handleClick}
      {...props}
    >
      {children}
    </Link>
  );
}
