// Shared mapping from a task status string to its glyph and CSS class.

export function statusGlyph(status: string): string {
  switch (status) {
    case 'done':
      return '●';
    case 'partial':
      return '◐';
    case 'inprogress':
      return '◔';
    case 'blocked':
      return '✕';
    default:
      return '○';
  }
}

export function statusClass(status: string): string {
  switch (status) {
    case 'done':
      return 'done';
    case 'partial':
      return 'partial';
    case 'inprogress':
      return 'inprog';
    case 'blocked':
      return 'blocked';
    default:
      return 'open';
  }
}
