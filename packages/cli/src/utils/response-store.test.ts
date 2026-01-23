import { describe, expect, test } from 'bun:test';
import type { ExecutionResult } from '../types/config';
import {
	createStoreContext,
	extractStoreValues,
	getValueByPath,
	mergeStoreContext,
	valueToString,
} from './response-store';

describe('getValueByPath', () => {
	const testObj = {
		status: 200,
		body: {
			id: 123,
			user: {
				name: 'John',
				email: 'john@example.com',
			},
			items: [
				{ id: 1, name: 'Item 1' },
				{ id: 2, name: 'Item 2' },
			],
		},
		headers: {
			'content-type': 'application/json',
			'x-request-id': 'abc123',
		},
	};

	test('should get top-level value', () => {
		expect(getValueByPath(testObj, 'status')).toBe(200);
	});

	test('should get nested value', () => {
		expect(getValueByPath(testObj, 'body.id')).toBe(123);
		expect(getValueByPath(testObj, 'body.user.name')).toBe('John');
		expect(getValueByPath(testObj, 'body.user.email')).toBe('john@example.com');
	});

	test('should get header value', () => {
		expect(getValueByPath(testObj, 'headers.content-type')).toBe('application/json');
		expect(getValueByPath(testObj, 'headers.x-request-id')).toBe('abc123');
	});

	test('should get array element by index', () => {
		expect(getValueByPath(testObj, 'body.items.0.id')).toBe(1);
		expect(getValueByPath(testObj, 'body.items.1.name')).toBe('Item 2');
	});

	test('should get array element using bracket notation', () => {
		expect(getValueByPath(testObj, 'body.items[0].id')).toBe(1);
		expect(getValueByPath(testObj, 'body.items[1].name')).toBe('Item 2');
	});

	test('should return undefined for non-existent path', () => {
		expect(getValueByPath(testObj, 'body.nonexistent')).toBeUndefined();
		expect(getValueByPath(testObj, 'body.user.age')).toBeUndefined();
		expect(getValueByPath(testObj, 'nonexistent.path')).toBeUndefined();
	});

	test('should return undefined for null or undefined object', () => {
		expect(getValueByPath(null, 'any.path')).toBeUndefined();
		expect(getValueByPath(undefined, 'any.path')).toBeUndefined();
	});

	test('should handle primitive values correctly', () => {
		expect(getValueByPath('string', 'length')).toBeUndefined();
		expect(getValueByPath(123, 'toString')).toBeUndefined();
	});
});

describe('valueToString', () => {
	test('should convert string to string', () => {
		expect(valueToString('hello')).toBe('hello');
	});

	test('should convert number to string', () => {
		expect(valueToString(123)).toBe('123');
		expect(valueToString(45.67)).toBe('45.67');
	});

	test('should convert boolean to string', () => {
		expect(valueToString(true)).toBe('true');
		expect(valueToString(false)).toBe('false');
	});

	test('should convert null and undefined to empty string', () => {
		expect(valueToString(null)).toBe('');
		expect(valueToString(undefined)).toBe('');
	});

	test('should JSON stringify objects', () => {
		expect(valueToString({ a: 1 })).toBe('{"a":1}');
	});

	test('should JSON stringify arrays', () => {
		expect(valueToString([1, 2, 3])).toBe('[1,2,3]');
	});
});

describe('extractStoreValues', () => {
	const mockResult: ExecutionResult = {
		request: {
			url: 'https://api.example.com/users',
			method: 'POST',
		},
		success: true,
		status: 201,
		headers: {
			'content-type': 'application/json',
			'x-request-id': 'req-12345',
		},
		body: {
			id: 456,
			data: {
				token: 'jwt-token-here',
				user: {
					id: 789,
					name: 'Test User',
				},
			},
		},
		metrics: {
			duration: 150,
			size: 1024,
		},
	};

	test('should extract status', () => {
		const result = extractStoreValues(mockResult, {
			statusCode: 'status',
		});
		expect(result.statusCode).toBe('201');
	});

	test('should extract body fields', () => {
		const result = extractStoreValues(mockResult, {
			userId: 'body.id',
			token: 'body.data.token',
			userName: 'body.data.user.name',
		});
		expect(result.userId).toBe('456');
		expect(result.token).toBe('jwt-token-here');
		expect(result.userName).toBe('Test User');
	});

	test('should extract header values', () => {
		const result = extractStoreValues(mockResult, {
			contentType: 'headers.content-type',
			requestId: 'headers.x-request-id',
		});
		expect(result.contentType).toBe('application/json');
		expect(result.requestId).toBe('req-12345');
	});

	test('should extract metrics', () => {
		const result = extractStoreValues(mockResult, {
			duration: 'metrics.duration',
		});
		expect(result.duration).toBe('150');
	});

	test('should handle non-existent paths', () => {
		const result = extractStoreValues(mockResult, {
			missing: 'body.nonexistent',
		});
		expect(result.missing).toBe('');
	});

	test('should extract multiple values', () => {
		const result = extractStoreValues(mockResult, {
			id: 'body.id',
			status: 'status',
			contentType: 'headers.content-type',
		});
		expect(result.id).toBe('456');
		expect(result.status).toBe('201');
		expect(result.contentType).toBe('application/json');
	});
});

describe('createStoreContext', () => {
	test('should create empty context', () => {
		const context = createStoreContext();
		expect(context).toEqual({});
	});
});

describe('mergeStoreContext', () => {
	test('should merge contexts', () => {
		const existing = { a: '1', b: '2' };
		const newValues = { c: '3', d: '4' };
		const merged = mergeStoreContext(existing, newValues);
		expect(merged).toEqual({ a: '1', b: '2', c: '3', d: '4' });
	});

	test('should override existing values', () => {
		const existing = { a: '1', b: '2' };
		const newValues = { b: 'new', c: '3' };
		const merged = mergeStoreContext(existing, newValues);
		expect(merged).toEqual({ a: '1', b: 'new', c: '3' });
	});

	test('should not mutate original contexts', () => {
		const existing = { a: '1' };
		const newValues = { b: '2' };
		const merged = mergeStoreContext(existing, newValues);
		expect(existing).toEqual({ a: '1' });
		expect(newValues).toEqual({ b: '2' });
		expect(merged).toEqual({ a: '1', b: '2' });
	});
});
