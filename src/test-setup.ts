import '@testing-library/jest-dom/vitest';
import { afterEach } from 'vitest';
import { cleanup } from '@testing-library/svelte';

// Auto-cleanup DOM after each test
afterEach(() => {
  cleanup();
});
