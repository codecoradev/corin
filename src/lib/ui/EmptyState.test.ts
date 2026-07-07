import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import EmptyStateWrapper from './__tests__/EmptyStateWrapper.svelte';

describe('EmptyState', () => {
  it('renders title', () => {
    render(EmptyStateWrapper, { props: { title: 'No documents found' } });
    expect(screen.getByText('No documents found')).toBeInTheDocument();
  });

  it('renders subtitle when provided', () => {
    render(EmptyStateWrapper, {
      props: { title: 'Empty', subtitle: 'Create your first document' },
    });
    expect(screen.getByText('Create your first document')).toBeInTheDocument();
  });

  it('renders custom icon', () => {
    render(EmptyStateWrapper, { props: { icon: '📄', title: 'Test' } });
    expect(screen.getByText('📄')).toBeInTheDocument();
  });

  it('renders action slot when provided', () => {
    render(EmptyStateWrapper, { props: { title: 'Test', withAction: true } });
    expect(screen.getByRole('button', { name: 'Create' })).toBeInTheDocument();
  });

  it('does not render action when not provided', () => {
    render(EmptyStateWrapper, { props: { title: 'Test', withAction: false } });
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });
});
