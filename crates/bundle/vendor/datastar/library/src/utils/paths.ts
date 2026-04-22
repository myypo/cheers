import type { Path, Paths } from '@engine/types'
import { hasOwn } from '@utils/polyfills'

export const isPojo = (obj: any): obj is Record<string, any> =>
  obj !== null &&
  typeof obj === 'object' &&
  (Object.getPrototypeOf(obj) === Object.prototype ||
    Object.getPrototypeOf(obj) === null)

export const isEmpty = (obj: Record<string, any>): boolean => {
  for (const prop in obj) {
    if (hasOwn(obj, prop)) {
      return false
    }
  }
  return true
}

export const updateLeaves = (
  obj: Record<string, any>,
  fn: (oldValue: any) => any,
) => {
  for (const key in obj) {
    const val = obj[key]
    if (isPojo(val) || Array.isArray(val)) {
      updateLeaves(val, fn)
    } else {
      obj[key] = fn(val)
    }
  }
}

const throwInvalidPath = (path: string, message: string, index: number): never => {
  throw new Error(`Invalid path ${JSON.stringify(path)} at ${index}: ${message}`)
}

const ROOT_SEGMENT_RE = /^[A-Za-z_\d][\w-]*/u

const isBarePathSegment = (segment: string): boolean =>
  /^[A-Za-z_\d][\w-]*$/u.test(segment)

const parseQuotedSegment = (path: string, index: number): [string, number] => {
  let value = ''
  index++

  while (index < path.length) {
    const char = path[index]!
    if (char === '\\') {
      index++
      if (index >= path.length) {
        throwInvalidPath(path, 'unterminated escape sequence', index)
      }

      const escaped = path[index]!
      switch (escaped) {
        case '\\':
        case '\'':
          value += escaped
          break
        case 'n':
          value += '\n'
          break
        case 'r':
          value += '\r'
          break
        case 't':
          value += '\t'
          break
        case 'u': {
          const hex = path.slice(index + 1, index + 5)
          if (!/^[\da-fA-F]{4}$/u.test(hex)) {
            throwInvalidPath(path, 'expected four hex digits after \\u', index)
          }
          value += String.fromCharCode(Number.parseInt(hex, 16))
          index += 4
          break
        }
        default:
          throwInvalidPath(path, `unsupported escape ${JSON.stringify(`\\${escaped}`)}`, index - 1)
      }

      index++
      continue
    }

    if (char === "'") {
      return [value, index + 1]
    }

    value += char
    index++
  }

  throwInvalidPath(path, 'unterminated quoted segment', index)
}

export const parsePath = (path: string): Path => {
  path = path.trim()
  if (!path.length) {
    return []
  }

  const root = path.match(ROOT_SEGMENT_RE)?.[0]
  if (root == null) {
    throwInvalidPath(path, 'expected a root segment', 0)
  }

  const segments: Path = [root]
  let index = root.length

  while (index < path.length) {
    if (path[index] !== '[') {
      throwInvalidPath(path, "expected ['segment']", index)
    }

    if (path[index + 1] !== "'") {
      throwInvalidPath(path, "expected a single-quoted bracket segment", index + 1)
    }

    const [segment, nextIndex] = parseQuotedSegment(path, index + 1)
    if (path[nextIndex] !== ']') {
      throwInvalidPath(path, 'expected ] after quoted segment', nextIndex)
    }

    segments.push(segment)
    index = nextIndex + 1
  }

  return segments
}

const pushEscapedQuotedSegment = (buffer: string[], segment: string) => {
  buffer.push("['")
  for (const char of segment) {
    switch (char) {
      case '\\':
        buffer.push('\\\\')
        break
      case '\'':
        buffer.push("\\'")
        break
      case '\n':
        buffer.push('\\n')
        break
      case '\r':
        buffer.push('\\r')
        break
      case '\t':
        buffer.push('\\t')
        break
      default:
        if (/\p{C}/u.test(char)) {
          buffer.push(`\\u${char.charCodeAt(0).toString(16).padStart(4, '0')}`)
        } else {
          buffer.push(char)
        }
    }
  }
  buffer.push("']")
}

export const serializePath = (path: Path): string => {
  const out: string[] = []

  for (const segment of path) {
    if (!out.length && isBarePathSegment(segment)) {
      out.push(segment)
    } else {
      pushEscapedQuotedSegment(out, segment)
    }
  }

  return out.join('')
}

export const pathToObj = (paths: Paths): Record<string, any> => {
  const result: Record<string, any> = {}
  for (const [path, value] of paths) {
    const keys = [...path]
    const lastKey = keys.pop()!
    const obj = keys.reduce((acc, key) => (acc[key] ??= {}), result)
    obj[lastKey] = value
  }
  return result
}
