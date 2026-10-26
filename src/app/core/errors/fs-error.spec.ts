import { describeFsError, toFsError } from './fs-error';

describe('fs-error', () => {
  it('keeps known codes', () => {
    expect(toFsError({ code: 'not_found', message: 'x', path: '/a' }).code).toBe('not_found');
  });

  it('maps unknown codes and other values to unknown', () => {
    expect(toFsError({ code: 'what', message: 'x' }).code).toBe('unknown');
    expect(toFsError(new Error('boom'))).toEqual({ code: 'unknown', message: 'boom' });
    expect(toFsError('plain').message).toBe('plain');
  });

  it('describes every code without Rust messages', () => {
    const text = describeFsError({ code: 'io', message: 'os error 5' });
    expect(text).not.toContain('os error');
  });
});
