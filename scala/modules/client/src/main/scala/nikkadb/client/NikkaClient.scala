package nikkadb.client

import nikkadb.shared.*
import nikkadb.shared.Protocol.*
import nikkadb.shared.Serializable.given

import java.net.Socket
import scala.collection.mutable

enum NikkaType:
  case TypeInt, TypeString

enum NikkaTypeWrapper:
  case NikkaInt(v: Byte)
  case NikkaString(v: String)

final class NikkaClient(port: String):
  private val connection: Socket = new Socket("127.0.0.1", port.toInt)
  private val in = connection.getInputStream
  private val out = connection.getOutputStream

  def setString(key: String, value: String): Either[String, Unit] =
    val args = Serializable[List[String]].toBytes(List(key, value))
    val request = Request(Action.CREATE, ContentType.KeyValue(ContentType.NString), args)
    sendRequest(request)
    formResponse(in) match
      case Response.Success       => Right(())
      case Response.Error(m)      => Left(m)
      case _                      => throw new RuntimeException("broken request packet")

  def getString(key: String): Option[String] =
    val bytes = key.getBytes("UTF-8")
    if bytes.length > 255 then throw new RuntimeException("key name is too big")
    val args = bytes.length.toByte +: bytes
    val request = Request(Action.GET, ContentType.NString, args)
    sendRequest(request)
    formResponse(in) match
      case Response.ContentResponse(ContentType.NString, content) =>
        Some(Serializable[List[String]].fromBytes(content).head)
      case Response.ContentResponse(_, _) => None
      case _ => throw new RuntimeException("broken response packet")

  def setInt(key: String, value: Byte): Either[String, Unit] =
    val base = Serializable[List[String]].toBytes(List(key))
    val args = base ++ Array[Byte](1, value)
    val request = Request(Action.CREATE, ContentType.KeyValue(ContentType.NInt), args)
    sendRequest(request)
    formResponse(in) match
      case Response.Success  => Right(())
      case Response.Error(m) => Left(m)
      case _                 => throw new RuntimeException("broken request packet")

  def getInt(key: String): Option[Byte] =
    val bytes = key.getBytes("UTF-8")
    if bytes.length > 255 then throw new RuntimeException("key name is too big")
    val args = bytes.length.toByte +: bytes
    val request = Request(Action.GET, ContentType.NInt, args)
    sendRequest(request)
    formResponse(in) match
      case Response.ContentResponse(ContentType.NInt, content) => Some(content(0))
      case Response.ContentResponse(_, _)                       => None
      case _ => throw new RuntimeException("broken response packet")

  def remove(key: String): Either[String, Unit] =
    val bytes = key.getBytes("UTF-8")
    if bytes.length > 255 then throw new RuntimeException("key is too big to store")
    val args = bytes.length.toByte +: bytes
    val request = Request(Action.DELETE, ContentType.NString, args)
    sendRequest(request)
    formResponse(in) match
      case Response.Success  => Right(())
      case Response.Error(m) => Left(m)
      case _                 => throw new RuntimeException("broken request packet")

  def getRegex(regex: String): List[String] =
    val bytes = regex.getBytes("UTF-8")
    if bytes.length > 255 then throw new RuntimeException("argument is too big")
    val args = bytes.length.toByte +: bytes
    val request = Request(Action.REGEX, ContentType.NString, args)
    sendRequest(request)
    formResponse(in) match
      case Response.ContentResponse(ContentType.NVector(_), content) =>
        Serializable[List[String]].fromBytes(content)
      case Response.ContentResponse(_, _) => Nil
      case _ => throw new RuntimeException("broken response packet")

  def beginTransaction(): Unit =
    fireAndForget(Request(Action.TSTART, ContentType.NNone, Array.emptyByteArray))

  def sendTransaction(): Unit =
    fireAndForget(Request(Action.TEND, ContentType.NNone, Array.emptyByteArray))

  def eraseTransaction(): Unit =
    fireAndForget(Request(Action.TERASE, ContentType.NNone, Array.emptyByteArray))

  def abortTransaction(): Unit =
    fireAndForget(Request(Action.TDISCARD, ContentType.NNone, Array.emptyByteArray))

  def clearDatabase(): Unit =
    fireAndForget(Request(Action.CLEAR, ContentType.NNone, Array.emptyByteArray))

  def createDeque(key: String, dequeType: NikkaType): Either[String, Unit] =
    val bytes = key.getBytes("UTF-8")
    if bytes.length > 255 then throw new RuntimeException("deque name is too long")
    val args = bytes.length.toByte +: bytes

    val trueDequeType = dequeType match
      case NikkaType.TypeInt    => ContentType.NInt
      case NikkaType.TypeString => ContentType.NString

    val request = Request(
      Action.CREATE,
      ContentType.KeyValue(ContentType.NDeque(trueDequeType)),
      args
    )
    sendRequest(request)
    formResponse(in) match
      case Response.Success => Right(())
      case _                => Left("cannot create deque")

  def pushFirst(key: String, value: NikkaTypeWrapper): Either[String, Unit] =
    val request = NikkaClient.formPushRequest(key, value, Action.PUSHF)
    sendRequest(request)
    formResponse(in) match
      case Response.Success  => Right(())
      case Response.Error(m) => Left(m)
      case _                 => throw new RuntimeException("logic error")

  def popFirst[T: Serializable](key: String): Option[T] =
    val request = Request(Action.POPF, ContentType.NString, key.getBytes("UTF-8"))
    sendRequest(request)
    formResponse(in) match
      case Response.ContentResponse(ContentType.NString | ContentType.NInt, vec) =>
        Some(Serializable[T].fromBytes(vec))
      case _ => None

  def pushLast(key: String, value: NikkaTypeWrapper): Either[String, Unit] =
    val request = NikkaClient.formPushRequest(key, value, Action.PUSHL)
    sendRequest(request)
    formResponse(in) match
      case Response.Success  => Right(())
      case Response.Error(m) => Left(m)
      case _                 => throw new RuntimeException("logic error")

  def popLast[T: Serializable](key: String): Option[T] =
    val request = Request(Action.POPL, ContentType.NString, key.getBytes("UTF-8"))
    sendRequest(request)
    formResponse(in) match
      case Response.ContentResponse(ContentType.NString | ContentType.NInt, vec) =>
        Some(Serializable[T].fromBytes(vec))
      case _ => None

  private def sendRequest(request: Request): Unit =
    val bytes = formPacket(request)
    out.write(bytes)
    out.flush()

  private def fireAndForget(request: Request): Unit =
    sendRequest(request)
    formResponse(in) // discard

object NikkaClient:
  def apply(): NikkaClient = new NikkaClient("1402")
  def withPort(port: String): NikkaClient = new NikkaClient(port)

  private def formPushRequest(
      key: String,
      value: NikkaTypeWrapper,
      action: Action
  ): Request =
    value match
      case NikkaTypeWrapper.NikkaInt(int) =>
        val valueBytes: Array[Byte] = Array(int)
        val keyBytes = key.getBytes("UTF-8")
        val args = mutable.ArrayBuffer.empty[Byte]
        args += keyBytes.length.toByte
        args ++= keyBytes
        args += valueBytes.length.toByte
        args ++= valueBytes
        Request(action, ContentType.KeyValue(ContentType.NInt), args.toArray)

      case NikkaTypeWrapper.NikkaString(str) =>
        val valueBytes = str.getBytes("UTF-8")
        val keyBytes = key.getBytes("UTF-8")
        val args = mutable.ArrayBuffer.empty[Byte]
        args += keyBytes.length.toByte
        args ++= keyBytes
        args += valueBytes.length.toByte
        args ++= valueBytes
        Request(action, ContentType.KeyValue(ContentType.NString), args.toArray)
