/** What a customer sees when they open the link. */
export function ShareAside({ teamName }: { teamName: string }) {
  return (
    <aside
      aria-label="How customers open it"
      className="box-border flex max-w-[360px] min-w-0 flex-[1_1_280px] flex-col gap-3 rounded-[8px] border border-hairline p-5 max-lg:max-w-none"
    >
      <h2 className="m-0 text-[14px] leading-5 font-semibold">What customers see</h2>
      <p className="m-0 text-[12px] leading-[18px] text-fg-muted">
        The link opens a page with the collection name, {teamName}, the request count and the last
        update, plus:
      </p>
      <ul className="m-0 list-disc pl-4 text-[12px] leading-5 text-fg-muted">
        <li>Open in curl-runner studio</li>
        <li>Download the studio</li>
        <li>
          <span className="font-mono text-fg">curl-runner pull</span> for any OS
        </li>
      </ul>
      <p className="m-0 text-[12px] leading-[18px] text-fg-muted">
        They fill in the inputs, and secrets stay on their machine. When CI publishes a new version,
        the studio offers the update.
      </p>
    </aside>
  );
}
