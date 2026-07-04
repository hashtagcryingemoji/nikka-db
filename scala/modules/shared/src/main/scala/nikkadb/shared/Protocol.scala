package nikkadb.shared

import java.io.{DataInputStream, InputStream}
import java.net.Socket
import scala.collection.mutable

enum Response:
  case Success
  case Error(message: String)
  case ContentResponse(contentType: ContentType, content: Array[Byte])

final case class Request(
    action: Action,
    contentType: ContentType,
    args: Array[Byte]
)

object Protocol:
  import Serializable.given

  given Serializable[Response] with
    def toBytes(value: Response): Array[Byte] =
      val buf = mutable.ArrayBuffer.empty[Byte]
      value match
        case Response.Success =>
          buf += 9
        case Response.ContentResponse(ct, content) =>
          buf += 0
          buf += ContentType.toByte(ct)
          if content.length > 255 then
            throw new RuntimeException("content is too big to store")
          buf += content.length.toByte
          buf ++= content
        case Response.Error(message) =>
          buf += 1
          val bytes = message.getBytes("UTF-8")
          if bytes.length > 255 then
            throw new RuntimeException("message is too long to store")
          buf += bytes.length.toByte
          buf ++= bytes
      buf.toArray

    def fromBytes(packet: Array[Byte]): Response =
      packet(0) match
        case 0 =>
          val ct = ContentType.fromByte(packet(1))
          val contentLen = java.lang.Byte.toUnsignedInt(packet(2))
          Response.ContentResponse(ct, packet.slice(3, 3 + contentLen))
        case 1 =>
          val messageLen = java.lang.Byte.toUnsignedInt(packet(1))
          val message = new String(packet.slice(2, 2 + messageLen), "UTF-8")
          Response.Error(message)
        case 9 => Response.Success
        case _ => throw new RuntimeException("broken packet")

  given Serializable[Request] with
    def toBytes(value: Request): Array[Byte] =
      val buf = mutable.ArrayBuffer.empty[Byte]
      buf += value.action.code

      buf += ContentType.toByte(value.contentType)

      value.contentType match
        case ContentType.KeyValue(valueType) =>
          buf += ContentType.toByte(valueType)
          valueType match
            case ContentType.NDeque(inner) => buf += ContentType.toByte(inner)
            case _                         => ()
        case _ => ()

      if value.args.length > 255 then
        throw new RuntimeException("arg is too big to store")
      buf += value.args.length.toByte
      buf ++= value.args
      buf.toArray

    def fromBytes(packet: Array[Byte]): Request =
      var index = 0

      val action = Action.fromByte(packet(index))
      index += 1

      val rawCt = ContentType.fromByte(packet(index))
      val contentType = rawCt match
        case ContentType.KeyValue(_) =>
          index += 1
          val valueType = ContentType.fromByte(packet(index))
          val trueValueType = valueType match
            case ContentType.NDeque(_) =>
              index += 1
              ContentType.NDeque(ContentType.fromByte(packet(index)))
            case other => other
          ContentType.KeyValue(trueValueType)
        case other => other

      index += 2

      val args =
        if index <= packet.length then packet.slice(index, packet.length)
        else throw new RuntimeException("broken packet")

      Request(action, contentType, args)

  def formPacket[T: Serializable](content: T): Array[Byte] =
    val body = Serializable[T].toBytes(content)
    if body.length > 255 then throw new RuntimeException("paket is too big")
    val out = new Array[Byte](body.length + 1)
    out(0) = body.length.toByte
    System.arraycopy(body, 0, out, 1, body.length)
    out

  def formResponse(in: InputStream): Response =
    val len = in.read()
    if len < 0 then throw new RuntimeException("error occurred while reading a packet")
    val buffer = new Array[Byte](len)
    val dis = new DataInputStream(in)
    dis.readFully(buffer)
    Serializable[Response].fromBytes(buffer)

  def extractKeyValue[T: Serializable](content: Array[Byte]): (String, T) =
    var index = 0
    val kSize = java.lang.Byte.toUnsignedInt(content(index))
    index += 1
    val key = new String(content.slice(index, kSize + 1), "UTF-8")
    index = kSize + 1
    val vSize = java.lang.Byte.toUnsignedInt(content(index))
    index += 1
    (key, Serializable[T].fromBytes(content.slice(index, index + vSize)))
