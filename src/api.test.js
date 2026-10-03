import test from 'node:test';
import assert from 'node:assert/strict';
import { dayRange, localDate, safeUrl } from './api.js';
test('day bounds use local midnight, including leap days', () => {
  const range = dayRange('2024-02-29');
  assert.equal(localDate(new Date(range.from * 1000)), '2024-02-29');
  assert.equal(localDate(new Date(range.to * 1000)), '2024-03-01');
});
test('links reject executable and malformed URL schemes', () => {
  for (const value of [
    'javascript:alert(1)',
    'data:text/html,hello',
    null,
    'hello',
  ])
    assert.equal(safeUrl(value), null);
  assert.equal(safeUrl('https://example.com/a'), 'https://example.com/a');
});
