
# Table of Contents

1.  [TASKS LIST](#orgd9c9da9)
    1.  [Understand the OTEL contract for logs](#org200fcf7):SPIKE:KUBER:
    2.  [Unit test cases of configurations](#org78fd920):KUBER:
    3.  [CI setup](#orgee2eb3c):BALAJI:
    4.  [Create implementations of the Actix app server](#orgaf90d8a):KUBER:BALAJI:DISCUSS:
    5.  [Injest server implementation](#orgcfa5681)
    6.  [Query server implementation](#orgfe907a0)
    7.  [Scripts and utils (python preferred)](#orgb6e957c):BALAJI:
    8.  [Flientbit agent](#org3986431)
        1.  [Create a golang plugin](#orgfb6c266)
    9.  [EBPF](#orgb55c3d5)
    10. [Add support for maps to filter data](#orgc7948eb)
    11. [Datafusion based SQL server](#org577884d)
    12. [Actor models](#orgebfc72a)
    13. [Try to see of the query can be called from external clients.](#org3b80bc3)
    14. [Check if SQLAlchemy can be made to work with the Datafusion.](#org92e3954)
    15. [Create a custom table component in Apache superset](#orgf77e401):REACT:UI:
    16. [Explore web assembly for own UI](#org9f2abfa)
    17. [Self healing](#org853bbeb):POC:
    18. [AI analysis](#org6e75569)
2.  [Decisions](#org354bdfc)
    1.  [WAL file system](#orge142274)
    2.  [Design](#orgee81fb4)
    3.  [Challenges](#org3efa395)
        1.  [Atomic offset calculations](#org399df8a)
        2.  [Multi-log files / dividing log files into chunks.](#org9604ffd)
    4.  [Considerations](#orga06d9d7)



<a id="orgd9c9da9"></a>

# TASKS LIST


<a id="org200fcf7"></a>

## TODO Understand the OTEL contract for logs     :SPIKE:KUBER:

We should strive to reuse these formats, we need interoperability and creating our own agents would prove too costly.

-   [ ] Check how protobuf formats are translated into arrow columns


<a id="org78fd920"></a>

## TODO Unit test cases of configurations     :KUBER:

-   [ ] Unit tests for Flight server
-   [ ] Unit test for broadcast actor
-   [ ] Unit test for parsing actor
-   [ ] Unit test for WAL actor
-   [ ] IT test for WAL writer. Tests for validating the headers on a temporary file.
-   [ ] 


<a id="orgee2eb3c"></a>

## TODO CI setup     :BALAJI:

-   [ ] Create a CI script in rust for compiling the Flatbufs and Protobufs if any.
-   [ ] Create a make script


<a id="orgaf90d8a"></a>

## TODO Create implementations of the Actix app server     :KUBER:BALAJI:DISCUSS:

-   [ ] Discuss contracts for REST
-   [ ] Discuss contracts for Websockets
-   [ ] Discuss contracts for flights


<a id="orgcfa5681"></a>

## DONE Injest server implementation

-   [X] Create a tonic server
-   [X] Create a flight server


<a id="orgfe907a0"></a>

## TODO Query server implementation

-   [ ] Create a health endpoint
-   [ ] Create an add regex endpoint


<a id="orgb6e957c"></a>

## TODO Scripts and utils (python preferred)     :BALAJI:

-   [X] Script for performance testing of flight server
-   [X] Script for validation of blocks


<a id="org3986431"></a>

## TODO Flientbit agent


<a id="orgfb6c266"></a>

### TODO Create a golang plugin

It should create arrow flight batches and ship it over grpc to collector

-   [ ] [#A] Create a skeleton plugin
-   [ ] [#A] Read and print the data
-   [ ] Create a contract for http, grpc traffic.
-   [ ] Create proxy interceptors in the contract.
-   [ ] Create a batching function
-   [ ] Side channel metadata - correlation - IP address, host id etc.


<a id="orgb55c3d5"></a>

## TODO EBPF

-   [ ] Self monitoring
-   [ ] Metering of injest
-   [ ] Blocking IPs using XDP


<a id="orgc7948eb"></a>

## TODO Add support for maps to filter data


<a id="org577884d"></a>

## TODO Datafusion based SQL server


<a id="orgebfc72a"></a>

## TODO Actor models

-   [ ] Actor for managing regex patterns
-   [ ] Actor for parsing json and converting into arrow
-   [ ] Explore **RAYON** for parallelism, we need not use actors because json parsing does not hold state.Actor for sequential writing of data
-   [ ] Actor for caching the parsed arrow buffers


<a id="org3b80bc3"></a>

## TODO Try to see of the query can be called from external clients.


<a id="org92e3954"></a>

## TODO Check if SQLAlchemy can be made to work with the Datafusion.


<a id="orgf77e401"></a>

## TODO Create a custom table component in Apache superset     :REACT:UI:


<a id="org9f2abfa"></a>

## TODO Explore web assembly for own UI


<a id="org853bbeb"></a>

## TODO Self healing     :POC:


<a id="org6e75569"></a>

## TODO AI analysis

-   [ ] POC on ONXX for transfering of ML models over wire
-   [ ] POC for auto-encoders for anomaly detection.
-   [ ] POC for convoluted auto-encoders for anomaly detection.
-   [ ] ONXX runtimes in different languages.
-   


<a id="org354bdfc"></a>

# Decisions


<a id="orge142274"></a>

## WAL file system

We need a write ahead log based file system for 2 reasons.

-   Parquet files need to be created and pushed into iceberg schema. This must be batched to avoid small file problems.
-   Logs must be durable.


<a id="orgee81fb4"></a>

## Design

<!-- This HTML table template is generated by emacs 30.1 -->
<table border="1">
  <tr>
    <td align="left" valign="top">
      &nbsp;Offset&nbsp;Range&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
    </td>
    <td align="left" valign="top">
      &nbsp;Field&nbsp;Name&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
    </td>
    <td align="left" valign="top">
      &nbsp;Size&nbsp;(bytes)
    </td>
  </tr>
  <tr>
    <td align="left" valign="top">
      &nbsp;0x00&nbsp;-&nbsp;0x07&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;0x08&nbsp;-&nbsp;0x0F&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;0x10&nbsp;-&nbsp;0x11&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;0x12&nbsp;-&nbsp;0x13&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;0x14&nbsp;-&nbsp;0x17&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;0x18&nbsp;-&nbsp;0x1F&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;0x20&nbsp;-&nbsp;0x27&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;0x28&nbsp;-&nbsp;0x2F&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;0x30&nbsp;-&nbsp;0x3F&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
    </td>
    <td align="left" valign="top">
      &nbsp;Magic&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;Metadata&nbsp;Offset&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;Metadata&nbsp;Length&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;Reserved&nbsp;(padding)&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;Checksum&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;Reserve&nbsp;Offset&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;Reserve&nbsp;Length&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;Total&nbsp;Block&nbsp;Size&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;Padding&nbsp;(for&nbsp;64-byte&nbsp;hdr)&nbsp;&nbsp;
    </td>
    <td align="left" valign="top">
      &nbsp;8&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;8&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;2&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;2&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;4&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;8&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;8&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;8&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;16&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
    </td>
  </tr>
  <tr>
    <td align="left" valign="top">
      &nbsp;0x40&nbsp;-&nbsp;0x40+M-1&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;0x40+M&nbsp;-&nbsp;0x40+M+D-1&nbsp;&nbsp;<br />
      &nbsp;...&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
    </td>
    <td align="left" valign="top">
      &nbsp;Metadata&nbsp;Bytes&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;Arrow&nbsp;IPC&nbsp;Data&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;Reserved&nbsp;Area&nbsp;(optional)&nbsp;&nbsp;&nbsp;
    </td>
    <td align="left" valign="top">
      &nbsp;M&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;D&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<br />
      &nbsp;R&nbsp;(optional)
    </td>
  </tr>
</table>

Legend:

-   \`M\` = metadata length (from header)
-   \`D\` = data length (derived from file size - header - metadata - reserve)
-   \`R\` = reserve length (from header)


<a id="org3efa395"></a>

## Challenges

Writes can only be sequential, and sequential writes may not be able to fully saturate the modern SSDs,
For parallelization, there are a few options.


<a id="org399df8a"></a>

### Atomic offset calculations

We can calculate the total block required for a buffer and atomically store it. Next I/O request will get this offset and then add it&rsquo;s size
automically and save it.
However, the downside is that to calculate the size of `RecordBuffers` accurately, we need to copy the data over to a `vec![]` datastructure.
This means we have to create another copy of the data in memory, which may not be very effecient.


<a id="org9604ffd"></a>

### Multi-log files / dividing log files into chunks.

This approach of creating 4 log files of 1 Gb, instead of a single 4 Gb logfile, could allow the work to parallelize.
However, this approach may be more complex and needs more dilibrations. Recovery could become pretty complex.


<a id="orga06d9d7"></a>

## Considerations

-   [ ] Need support for vectorized search of block headers. We can recover in parallel and create parquets in parallel using rayon.
-   [ ] Need a way to parallelize writes.
-   [ ] Dataloss of a few seconds may be acceptable. (Flushing for every entry in WAL could be expensive, databases flush on commit, we can flush every 2 seconds maybe)
-   [ ] Check-sum and corruption checks must be done.
-   [ ] Metadata should be field extractable, and no incur full serialization. (Flatbuf allows for primitive types to be de-serialized without reading entire payload)
-   [ ] Need some paddings in the WAL buffer for SIMD accelerations.
-   [ ] Need some padding in the headers for future explansions.
-   [ ] Need schema evolution options for the Metadata
-   [ ] Need validation scripts for checking block health and checksums for troubleshootings.

