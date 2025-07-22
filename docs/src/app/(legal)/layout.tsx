import type { ReactNode } from "react";
import { Footer } from "@/components/layout/footer";

export default function LegalLayout({ children }: { children: ReactNode }) {
  return (
    <>
      {children}
      <Footer />
    </>
  );
}
