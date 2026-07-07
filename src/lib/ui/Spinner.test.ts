import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import Spinner from './Spinner.svelte';

describe('Spinner', () => {
  it('renders a spinner element', () => {
    render(Spinner);
    const spinner = document.querySelector('.spinner');
    expect(spinner).toBeInTheDocument();
  });

  it('applies custom size', () => {
    render(Spinner, { props: { size: 32 } });
    const spinner = document.querySelector('.spinner') as HTMLElement;
    expect(spinner.style.width).toBe('32px');
    expect(spinner.style.height).toBe('32px');
  });

  it('uses default size of 16px', () => {
    render(Spinner);
    const spinner = document.querySelector('.spinner') as HTMLElement;
    expect(spinner.style.width).toBe('16px');
  });
});
