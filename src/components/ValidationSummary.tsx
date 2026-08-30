export function ValidationSummary({
  title,
  items,
}: {
  title: string;
  items: string[];
}) {
  return (
    <section className="validation-summary" aria-live="polite">
      <h2>{title}</h2>
      <ul>
        {items.map((item) => (
          <li key={item}>{item}</li>
        ))}
      </ul>
    </section>
  );
}
