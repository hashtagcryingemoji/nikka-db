package nikkadb.server

import nikkadb.server.utils.TrieNode
import nikkadb.shared.{ContentType, Value}

import scala.collection.mutable

final class NikkaDb(
    val storage: mutable.Map[String, Value] = mutable.Map.empty,
    val trie: TrieNode = TrieNode()
):

  def add(key: String, value: Value): Unit =
    trie.insert(key)
    storage.update(key, value)

  def delete(key: String): Unit =
    trie.remove(key)
    storage.remove(key)

  def get(key: String): Option[Value] = storage.get(key)

  def findRegex(regex: String): List[String] = trie.findRegex(regex)

  def clear(): Unit = storage.clear()

  def popFirst(key: String): Option[Value] =
    val deque = storage.get(key) match
      case Some(d) => d
      case None    => return None

    val dequeType = deque._1
    val dequeContent = mutable.ArrayBuffer.from(deque._2)

    if dequeContent.isEmpty then return None

    dequeType match
      case ContentType.NDeque(contentType) =>
        val content: Array[Byte] = contentType match
          case ContentType.NString =>
            val len = java.lang.Byte.toUnsignedInt(dequeContent(0))
            val dirty = mutable.ArrayBuffer.empty[Byte]
            for _ <- 0 until (len + 2) do
              dirty += dequeContent.remove(0)
            dirty.remove(0)
            dirty.remove(dirty.length - 1)
            dirty.toArray
          case ContentType.NInt =>
            Array(dequeContent.remove(0))
          case _ => throw new RuntimeException("logic error")

        storage.update(
          key,
          (ContentType.NDeque(contentType), dequeContent.toArray)
        )
        Some((contentType, content))
      case _ => None

  def popLast(key: String): Option[Value] =
    val deque = storage.get(key) match
      case Some(d) => d
      case None    => return None

    val dequeType = deque._1
    val dequeContent = mutable.ArrayBuffer.from(deque._2)
    val dequeLen = dequeContent.length

    if dequeLen == 0 then return None

    dequeType match
      case ContentType.NDeque(contentType) =>
        val content: Array[Byte] = contentType match
          case ContentType.NInt =>
            val b = dequeContent.remove(dequeLen - 1)
            Array(b)
          case ContentType.NString =>
            val len = java.lang.Byte.toUnsignedInt(dequeContent(dequeLen - 1))
            val start = dequeLen - len - 2
            val dirty = mutable.ArrayBuffer.empty[Byte]
            for i <- start until dequeLen do dirty += dequeContent(i)
            dequeContent.remove(start, dequeLen - start)
            dirty.remove(0)
            dirty.remove(dirty.length - 1)
            dirty.toArray
          case _ => throw new RuntimeException("logic error")

        storage.update(
          key,
          (ContentType.NDeque(contentType), dequeContent.toArray)
        )
        Some((contentType, content))
      case _ => None
