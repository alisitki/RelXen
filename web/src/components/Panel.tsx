import type { PropsWithChildren } from "react";

export function Panel({ title, children }: PropsWithChildren<{ title: string }>) {
  return (
    <section className="panel">
      <header className="panel__header">
        <span>{title}</span>
      </header>
      <div className="panel__body">{children}</div>
    </section>
  );
}
