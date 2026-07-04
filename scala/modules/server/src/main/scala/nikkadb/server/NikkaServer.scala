package nikkadb.server

import nikkadb.server.utils.TrieNode
import nikkadb.shared.*
import nikkadb.shared.Protocol.*
import nikkadb.shared.Serializable.given

import java.io.{BufferedInputStream, File, FileInputStream, FileOutputStream, IOException, PushbackInputStream}
import java.net.{InetSocketAddress, ServerSocket, Socket}
import java.nio.file.{Files, Path}
import java.util.concurrent.{ConcurrentLinkedQueue, LinkedBlockingQueue}
import java.util.concurrent.atomic.AtomicBoolean
import scala.collection.mutable

final class NikkaServer(port: String):
  val serverSocket: ServerSocket =
    val ss = new ServerSocket()
    ss.bind(new InetSocketAddress("127.0.0.1", port.toInt))
    ss

  private val logFile: File = openOrCreate("log")
  private val backupFile: File = openOrCreate("backup")

  private val backupNotifier = new LinkedBlockingQueue[Boolean]()
  private val backupAlive = new AtomicBoolean(true)

  private val storage: mutable.Map[String, Value] =
    val bytes = Files.readAllBytes(backupFile.toPath)
    if bytes.isEmpty then mutable.Map.empty
    else Serializable[mutable.Map[String, Value]].fromBytes(bytes)

  private val trie: TrieNode =
    val t = TrieNode()
    for k <- storage.keys do t.insert(k)
    t

  private val database: NikkaDb = new NikkaDb(storage, trie)
  private val clients: mutable.ArrayBuffer[Client] = mutable.ArrayBuffer.empty
  private val lock: AnyRef = new Object
  private var backupCounter: Int = 0

  private def openOrCreate(name: String): File =
    val f = new File(name)
    if !f.exists() then f.createNewFile()
    f

  def run(): Unit =
    val incomingSockets = new ConcurrentLinkedQueue[Socket]()

    val backupThread = new Thread(() => backupControl(), "nikka-backup")
    backupThread.setDaemon(true)
    backupThread.start()

    val acceptor = new Thread(
      () =>
        try
          while true do
            val socket = serverSocket.accept()
            socket.setSoTimeout(1) // non-blocking-ish
            incomingSockets.add(socket)
        catch case _: IOException => (),
      "nikka-acceptor"
    )
    acceptor.setDaemon(true)
    acceptor.start()

    while true do
      var s = incomingSockets.poll()
      while s != null do
        clients += new Client(s)
        s = incomingSockets.poll()

      var i = clients.length - 1
      while i >= 0 do
        val client = clients(i)
        val socket = client.socket
        val input =
          try Some(new PushbackInputStream(socket.getInputStream, 1))
          catch case _: IOException => None

        val dataVec = mutable.ArrayBuffer.empty[Array[Byte]]
        var disconnected = false
        var continueOuter = false

        input match
          case None =>
            clients.remove(i)
          case Some(in) =>
            var harvesting = true
            while harvesting do
              try
                val first = in.read()
                if first == -1 then
                  clients.remove(i)
                  disconnected = true
                  continueOuter = true
                  harvesting = false
                else
                  val requestLen = first & 0xff
                  val body = new Array[Byte](requestLen)
                  var got = 0
                  var eof = false
                  while got < requestLen && !eof do
                    val n = in.read(body, got, requestLen - got)
                    if n == -1 then eof = true
                    else got += n
                  if eof then
                    clients.remove(i)
                    disconnected = true
                    continueOuter = true
                    harvesting = false
                  else
                    dataVec += body
              catch
                case _: java.net.SocketTimeoutException =>
                  if dataVec.isEmpty then
                    Thread.sleep(100)
                    continueOuter = true
                  harvesting = false
                case _: IOException =>
                  throw new RuntimeException()

            if !continueOuter then
              for bytes <- dataVec do
                val request = Serializable[Request].fromBytes(bytes)

                if client.state == ClientState.TRANSACTION &&
                  request.action != Action.TDISCARD &&
                  request.action != Action.TEND &&
                  request.action != Action.TERASE
                then
                  client.queue.enqueue(request)
                  val responseBytes = formPacket(Response.Success)
                  socket.getOutputStream.write(responseBytes)
                  socket.getOutputStream.flush()
                else
                  val response = client.processAction(request, lock, database)
                  val responseBytes = formPacket(response)
                  socket.getOutputStream.write(responseBytes)
                  socket.getOutputStream.flush()

                  backupCounter += 1
                  if backupCounter >= 100 then
                    backupNotifier.offer(true)

        i -= 1

  private def backupControl(): Unit =
    while backupAlive.get() do
      val signal = backupNotifier.poll()
      if signal != null then
        val bytes = lock.synchronized {
          Serializable[mutable.Map[String, Value]].toBytes(database.storage)
        }
        val out = new FileOutputStream(backupFile, false)
        try
          out.write(bytes)
          out.flush()
        finally out.close()
      else
        try Thread.sleep(0, 100000) // 100 microseconds ~
        catch case _: InterruptedException => backupAlive.set(false)

object NikkaServer:
  def apply(): NikkaServer = new NikkaServer("1402")
  def withPort(port: String): NikkaServer = new NikkaServer(port)
