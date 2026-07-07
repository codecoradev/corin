import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import BadgeWrapper from './__tests__/BadgeWrapper.svelte';

describe('Badge', () => {
  it('renders text content', () => {
    render(BadgeWrapper, { props: { text: 'important' } });
    expect(screen.getByText('important')).toBeInTheDocument();
  });

  it('applies default color class', () => {
    render(BadgeWrapper, { props: { color: 'default', text: 'tag' } });
    expect(screen.getByText('tag').closest('.badge')).toHaveClass('badge-default');
  });

  it('applies accent color class', () => {
    render(BadgeWrapper, { props: { color: 'accent', text: 'tag' } });
    expect(screen.getByText('tag').closest('.badge')).toHaveClass('badge-accent');
  });

  it('applies green color class', () => {
    render(BadgeWrapper, { props: { color: 'green', text: 'tag' } });
    expect(screen.getByText('tag').closest('.badge')).toHaveClass('badge-green');
  });

  it('applies red color class', () => {
    render(BadgeWrapper, { props: { color: 'red', text: 'tag' } });
    expect(screen.getByText('tag').closest('.badge')).toHaveClass('badge-red');
  });
});
