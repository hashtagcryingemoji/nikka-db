package nikkadb.shared

import scala.collection.mutable

enum ContentType:
  case NNone
  case NString
  case NInt
  case KeyValue(inner: ContentType)
  case NVector(inner: ContentType)
  case NDeque(inner: ContentType)

object ContentType:
  def toByte(ct: ContentType): Byte = ct match
    case NNone       => 0
    case NString     => 1
    case NInt        => 2
    case KeyValue(_) => 3
    case NVector(_)  => 4
    case NDeque(_)   => 5

  def fromByte(b: Byte): ContentType = b match
    case 0 => NNone
    case 1 => NString
    case 2 => NInt
    case 3 => KeyValue(NNone)
    case 4 => NVector(NNone)
    case 5 => NDeque(NNone)
    case _ => throw new RuntimeException("conversion error")

enum Action(val code: Byte):
  case CREATE   extends Action(1)
  case DELETE   extends Action(2)
  case GET      extends Action(3)
  case REGEX    extends Action(4)
  case TSTART   extends Action(5)
  case TEND     extends Action(6)
  case TERASE   extends Action(7)
  case TDISCARD extends Action(8)
  case CLEAR    extends Action(9)
  case POPL     extends Action(10)
  case POPF     extends Action(11)
  case PUSHL    extends Action(12)
  case PUSHF    extends Action(13)

object Action:
  def fromByte(b: Byte): Action = b match
    case 1  => CREATE
    case 2  => DELETE
    case 3  => GET
    case 4  => REGEX
    case 5  => TSTART
    case 6  => TEND
    case 7  => TERASE
    case 8  => TDISCARD
    case 9  => CLEAR
    case 10 => POPL
    case 11 => POPF
    case 12 => PUSHL
    case 13 => PUSHF
    case _  => throw new RuntimeException("conversion error")

/** (ContentType, Vec<u8>) alias */
type Value = (ContentType, Array[Byte])

/** Type class mirroring Rust's `Serializable` trait. */
trait Serializable[T]:
  def toBytes(value: T): Array[Byte]
  def fromBytes(content: Array[Byte]): T

object Serializable:
  def apply[T](using s: Serializable[T]): Serializable[T] = s

  extension [T](value: T)(using s: Serializable[T])
    def toBytes: Array[Byte] = s.toBytes(value)

  given Serializable[String] with
    def toBytes(value: String): Array[Byte] = value.getBytes("UTF-8")
    def fromBytes(content: Array[Byte]): String = new String(content, "UTF-8")

  given Serializable[Byte] with
    def toBytes(value: Byte): Array[Byte] = Array(value)
    def fromBytes(content: Array[Byte]): Byte = content(0)

  given Serializable[List[String]] with
    def toBytes(value: List[String]): Array[Byte] =
      val buf = mutable.ArrayBuffer.empty[Byte]
      for content <- value do
        val bytes = content.getBytes("UTF-8")
        if bytes.length > 255 then
          throw new RuntimeException("cannot support content that long")
        buf += bytes.length.toByte
        buf ++= bytes
      buf.toArray

    def fromBytes(content: Array[Byte]): List[String] =
      val buf = mutable.ListBuffer.empty[String]
      var index = 0
      while index < content.length do
        val size = java.lang.Byte.toUnsignedInt(content(index))
        index += 1
        val slice = content.slice(index, index + size)
        index += size
        buf += new String(slice, "UTF-8")
      buf.toList

  /** HashMap<String, Value> — matches the Rust `impl Serializable for HashMap`. */
  given Serializable[mutable.Map[String, Value]] with
    def toBytes(value: mutable.Map[String, Value]): Array[Byte] =
      val buf = mutable.ArrayBuffer.empty[Byte]
      for (k, v) <- value do
        val kBytes = k.getBytes("UTF-8")
        if kBytes.length > 255 then
          throw new RuntimeException("cannot support size that big yet")
        buf += kBytes.length.toByte
        buf ++= kBytes

        val contentByte = ContentType.toByte(v._1)
        buf += contentByte
        v._1 match
          case ContentType.NDeque(nested) => buf += ContentType.toByte(nested)
          case _                          => ()

        if v._2.length > 255 then
          throw new RuntimeException("cannot support size that big yet")
        buf += v._2.length.toByte
        buf ++= v._2
      buf.toArray

    def fromBytes(content: Array[Byte]): mutable.Map[String, Value] =
      val hm = mutable.Map.empty[String, Value]
      var index = 0
      while index < content.length do
        val size = java.lang.Byte.toUnsignedInt(content(index))
        println(s"size: $size")
        index += 1
        val k = content.slice(index, index + size)
        index += size

        var contentType = ContentType.fromByte(content(index))
        index += 1
        contentType match
          case ContentType.NDeque(_) =>
            val nested = ContentType.fromByte(content(index))
            contentType = ContentType.NDeque(nested)
            index += 1
          case _ => ()

        val vSize = java.lang.Byte.toUnsignedInt(content(index))
        index += 1
        val v = content.slice(index, index + vSize)
        index += vSize

        hm += (new String(k, "UTF-8") -> (contentType, v))
      hm
