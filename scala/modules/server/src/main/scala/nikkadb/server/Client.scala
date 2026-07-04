package nikkadb.server

import nikkadb.shared.*
import nikkadb.shared.Protocol.*
import nikkadb.shared.Serializable.given

import java.net.Socket
import scala.collection.mutable

enum ClientState:
  case DEFAULT, TRANSACTION

final class Client(
    val socket: Socket,
    var state: ClientState = ClientState.DEFAULT,
    val queue: mutable.Queue[Request] = mutable.Queue.empty
):

  def processAction(request: Request, lock: AnyRef, db: NikkaDb): Response =
    val action = request.action
    val args = request.args

    action match
      case Action.GET =>
        processGetRequest(lock, db, args, request.contentType) match
          case Right(v) => v
          case Left(v)  => v

      case Action.CREATE =>
        val (k, v) = Client.extractSerializedKeyValue(args, request.contentType)
        lock.synchronized { db.add(k, v) }
        Response.Success

      case Action.DELETE =>
        val key = Serializable[List[String]].fromBytes(args).head
        lock.synchronized { db.delete(key) }
        Response.Success

      case Action.REGEX =>
        val regex = Serializable[List[String]].fromBytes(args).head
        val content = lock.synchronized {
          Serializable[List[String]].toBytes(db.findRegex(regex))
        }
        Response.ContentResponse(
          ContentType.NVector(ContentType.NString),
          content
        )

      case Action.TSTART =>
        state = ClientState.TRANSACTION
        Response.Success

      case Action.TEND =>
        state = ClientState.DEFAULT
        processTransaction(lock, db)

      case Action.TERASE =>
        queue.clear()
        Response.Success

      case Action.TDISCARD =>
        state = ClientState.DEFAULT
        queue.clear()
        Response.Success

      case Action.CLEAR =>
        lock.synchronized { db.clear() }
        Response.Success

      case Action.POPF =>
        val key = new String(args, "UTF-8")
        val value = lock.synchronized { db.popFirst(key) }
        value match
          case Some(v) => Response.ContentResponse(v._1, v._2)
          case None    => Response.ContentResponse(ContentType.NNone, Array.emptyByteArray)

      case Action.POPL =>
        val key = new String(args, "UTF-8")
        val value = lock.synchronized { db.popLast(key) }
        value match
          case Some(v) => Response.ContentResponse(v._1, v._2)
          case None    => Response.ContentResponse(ContentType.NNone, Array.emptyByteArray)

      case Action.PUSHF =>
        lock.synchronized {
          val (valueBytes, key, deque) = Client.getDequeAndPushValue(args, db)
          deque match
            case None => Response.Error("invalid key for deque")
            case Some(value) =>
              request.contentType match
                case ContentType.KeyValue(dequeType) =>
                  if value._1 != ContentType.NDeque(dequeType) then
                    Response.Error("invalid key for deque")
                  else
                    dequeType match
                      case ContentType.NInt =>
                        val merged = valueBytes ++ value._2
                        db.add(key, (value._1, merged))
                        Response.Success

                      case ContentType.NString =>
                        val sep = valueBytes.length.toByte
                        val wrapped = (sep +: valueBytes) :+ sep
                        val merged = wrapped ++ value._2
                        db.add(key, (value._1, merged))
                        Response.Success

                      case _ => throw new RuntimeException("logic error")
                case _ => throw new RuntimeException("logic error")
        }

      case Action.PUSHL =>
        lock.synchronized {
          val (valueBytes, key, deque) = Client.getDequeAndPushValue(args, db)
          deque match
            case None => Response.Error("invalid key for deque")
            case Some(value) =>
              request.contentType match
                case ContentType.KeyValue(dequeType) =>
                  if value._1 != ContentType.NDeque(dequeType) then
                    Response.Error("invalid key for deque")
                  else
                    dequeType match
                      case ContentType.NInt =>
                        val merged = value._2 ++ valueBytes
                        db.add(key, (value._1, merged))
                        Response.Success

                      case ContentType.NString =>
                        val sep = valueBytes.length.toByte
                        val wrapped = (sep +: valueBytes) :+ sep
                        val merged = value._2 ++ wrapped
                        db.add(key, (value._1, merged))
                        Response.Success

                      case _ => throw new RuntimeException("logic error")
                case _ => throw new RuntimeException("logic error")
        }

  private def processGetRequest(
      lock: AnyRef,
      db: NikkaDb,
      args: Array[Byte],
      contentType: ContentType
  ): Either[Response, Response] =
    val key = Serializable[List[String]].fromBytes(args).head
    lock.synchronized {
      contentType match
        case ContentType.NString =>
          db.get(key) match
            case Some(value) =>
              val v = List(new String(value._2, "UTF-8"))
              Right(
                Response.ContentResponse(
                  ContentType.NString,
                  Serializable[List[String]].toBytes(v)
                )
              )
            case None =>
              Right(Response.ContentResponse(ContentType.NNone, Array.emptyByteArray))
        case ContentType.NInt =>
          db.get(key) match
            case Some(value) =>
              if value._1 != ContentType.NInt then
                Left(Response.Error("invalid key for string"))
              else
                Right(Response.ContentResponse(ContentType.NInt, Array(value._2(0))))
            case None =>
              Right(Response.ContentResponse(ContentType.NNone, Array.emptyByteArray))
        case _ => throw new RuntimeException("logic error")
    }

  private def processTransaction(lock: AnyRef, db: NikkaDb): Response =
    lock.synchronized {
      val snapshot = mutable.Map.from(db.storage)
      for request <- queue do
        Client.processInTransaction(request, snapshot)
      db.storage.clear()
      db.storage ++= snapshot
    }
    Response.Success

object Client:

  def processInTransaction(
      request: Request,
      snapshot: mutable.Map[String, Value]
  ): Response =
    val action = request.action
    val args = request.args

    action match
      case Action.CREATE =>
        val items = Serializable[List[String]].fromBytes(args)
        items match
          case key :: value :: _ =>
            snapshot.update(key, (ContentType.NString, value.getBytes("UTF-8")))
          case _ => throw new RuntimeException("incorrect request")
        Response.Success

      case Action.DELETE =>
        val key = Serializable[List[String]].fromBytes(args).head
        snapshot.remove(key)
        Response.Success

      case _ => throw new RuntimeException("logic error")

  def extractSerializedKeyValue(
      args: Array[Byte],
      contentType: ContentType
  ): (String, Value) =
    contentType match
      case ContentType.KeyValue(valueType) =>
        valueType match
          case ContentType.NString =>
            val (k, v) = extractKeyValue[String](args)
            (k, (ContentType.NString, v.getBytes("UTF-8")))
          case ContentType.NInt =>
            val (k, v) = extractKeyValue[Byte](args)
            (k, (ContentType.NInt, Array(v)))
          case ContentType.NDeque(inner) =>
            val size = java.lang.Byte.toUnsignedInt(args(0))
            val key = new String(args.slice(1, size + 1), "UTF-8")
            (key, (ContentType.NDeque(inner), Array.emptyByteArray))
          case _ => throw new RuntimeException("logic error")
      case _ => throw new RuntimeException("logic error")

  def getDequeAndPushValue(
      args: Array[Byte],
      db: NikkaDb
  ): (Array[Byte], String, Option[Value]) =
    val keySize = java.lang.Byte.toUnsignedInt(args(0))
    val keyBytes = args.slice(1, keySize + 1)

    val valueSize = java.lang.Byte.toUnsignedInt(args(keySize + 1))
    val valueBytes = args.slice(keySize + 2, keySize + 2 + valueSize)

    val key = new String(keyBytes, "UTF-8")
    val deque = db.get(key)
    (valueBytes, key, deque)
