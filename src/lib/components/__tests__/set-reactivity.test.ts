import { describe, it, expect } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import TreeRepro from './TreeRepro.svelte';

describe('doc-tree toggle: $state Set needs immutable update', () => {
  it('toggles open/closed on click', async () => {
    const { getByTestId, queryByTestId } = render(TreeRepro);
    // starts expanded
    expect(queryByTestId('kids-a')).toBeTruthy();
    await fireEvent.click(getByTestId('toggle-a'));
    expect(queryByTestId('kids-a')).toBeNull();
    await fireEvent.click(getByTestId('toggle-a'));
    expect(queryByTestId('kids-a')).toBeTruthy();
  });
});
