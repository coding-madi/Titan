
# Table of Contents

1.  [TASKS LIST](#org9754111)
    1.  [Understand the OTEL contract for logs](#org431e818):SPIKE:
    2.  [Check how protobuf formats are translated into arrow columns](#org26a8cb4)
    3.  [Create a `build.rs` script with protobuf compilation and generations.](#org7aa2cd9)
    4.  [Unit test cases of configurations](#org0a566a4):KUBER:
    5.  [Create implementations of the Actix app server](#org555460a)
    6.  [Injest server implementation](#orge4f7ad7)
        1.  [Create a tonic server](#orgead04a1)
        2.  [Create a flight server](#org8a22645)
    7.  [Query server implementation](#org184957a)
        1.  [Create a health endpoint](#org298b92d)
        2.  [Create an add regex endpoint](#orge005f91)
    8.  [Datafusion based SQL server](#org3700883)
    9.  [datafusion-flight-sql-server](#orgf02e02f)
    10. [Flientbit agent](#org1375d0c)
        1.  [Create a golang plugin](#orga00582f)
    11. [Create ebpf example in aya for XDP packet filtering](#org680e7d6)
    12. [Add support for maps to filter data](#orgb42f525)
    13. [Add support for actix web application.](#orgf9a1ec4)
    14. [Create a CI pipeline and build script](#orgc6b7e02)
    15. [Create a build.rs script for protobuf generations](#org2eaff83)
    16. [Create a actions script for building pipeline.](#org038d7ac)
    17. [Create a make script](#org9e40ec9)
    18. [Create a script for packaging into different distributions.](#org33bae24)
    19. [Actor models](#org63eede8)
    20. [Actor for managing regex patterns](#org464abc8)
    21. [Actor for parsing json and converting into arrow](#orgd0bf425)
        1.  [Explore **RAYON** for parallelism, we need not use actors because json parsing does not hold state.](#org0a7c1c8)
    22. [Actor for sequential writing of data](#org88a0bea)
    23. [Actor for caching the parsed arrow buffers](#org53adba7)
    24. [](#orgb3910db)
    25. [Datafusion](#org20640e4)
    26. [Query the data using datafusion.](#orge9dfb82)
    27. [Try to see of the query can be called from external clients.](#org783e360)
    28. [Check if SQLAlchemy can be made to work with the Datafusion.](#orga7c5b23)
    29. [Create a custom table component in Apache superset](#org1120392):REACT:UI:
    30. [Explore web assembly for own UI](#org3a793b7)
    31. [AI based analysis](#org3f6aa48)
    32. [Self healing](#org9617094):POC:
    33. [Edge analysis](#org21380fb)
    34. [Analyse the gather already gathered and get some](#org654cf2d)
    35. [](#org74a3ed4)
2.  [Decisions](#orgef3dfec)
    1.  [WAL file system](#org02a31ed)
    2.  [Design](#org94bdc2d)
    3.  [Challenges](#org8c136e0)
        1.  [Atomic offset calculations](#orgf74e1e2)
        2.  [Multi-log files / dividing log files into chunks.](#orgd43ab5a)
    4.  [Considerations](#orged2be23)



<a id="org9754111"></a>

# TASKS LIST


<a id="org431e818"></a>

## TODO Understand the OTEL contract for logs     :SPIKE:

We should strive to reuse these formats, we need interoperability and creating our own agents would prove too costly.


<a id="org26a8cb4"></a>

## TODO Check how protobuf formats are translated into arrow columns


<a id="org7aa2cd9"></a>

## TODO Create a `build.rs` script with protobuf compilation and generations.


<a id="org0a566a4"></a>

## TODO Unit test cases of configurations     :KUBER:


<a id="org555460a"></a>

## TODO Create implementations of the Actix app server


<a id="orge4f7ad7"></a>

## TODO Injest server implementation


<a id="orgead04a1"></a>

### TODO Create a tonic server


<a id="org8a22645"></a>

### TODO Create a flight server


<a id="org184957a"></a>

## TODO Query server implementation


<a id="org298b92d"></a>

### TODO Create a health endpoint


<a id="orge005f91"></a>

### TODO Create an add regex endpoint


<a id="org3700883"></a>

## TODO Datafusion based SQL server


<a id="orgf02e02f"></a>

## TODO datafusion-flight-sql-server


<a id="org1375d0c"></a>

## TODO Flientbit agent


<a id="orga00582f"></a>

### TODO Create a golang plugin

It should create arrow flight batches and ship it over grpc to collector

1.  TODO Create a skeleton plugin

2.  TODO Read and print the data

3.  TODO Create a contract for http, grpc traffic.

4.  TODO Create proxy interceptors in the contract.

5.  TODO Create a batching function

6.  TODO Side channel metadata - correlation - IP address, host id etc.


<a id="org680e7d6"></a>

## TODO Create ebpf example in aya for XDP packet filtering


<a id="orgb42f525"></a>

## TODO Add support for maps to filter data


<a id="orgf9a1ec4"></a>

## TODO Add support for actix web application.


<a id="orgc6b7e02"></a>

## TODO Create a CI pipeline and build script


<a id="org2eaff83"></a>

## TODO Create a build.rs script for protobuf generations


<a id="org038d7ac"></a>

## TODO Create a actions script for building pipeline.


<a id="org9e40ec9"></a>

## TODO Create a make script


<a id="org33bae24"></a>

## TODO Create a script for packaging into different distributions.


<a id="org63eede8"></a>

## TODO Actor models


<a id="org464abc8"></a>

## TODO Actor for managing regex patterns


<a id="orgd0bf425"></a>

## TODO Actor for parsing json and converting into arrow


<a id="org0a7c1c8"></a>

### TODO Explore **RAYON** for parallelism, we need not use actors because json parsing does not hold state.


<a id="org88a0bea"></a>

## TODO Actor for sequential writing of data


<a id="org53adba7"></a>

## TODO Actor for caching the parsed arrow buffers


<a id="orgb3910db"></a>

## TODO 


<a id="org20640e4"></a>

## TODO Datafusion


<a id="orge9dfb82"></a>

## TODO Query the data using datafusion.


<a id="org783e360"></a>

## TODO Try to see of the query can be called from external clients.


<a id="orga7c5b23"></a>

## TODO Check if SQLAlchemy can be made to work with the Datafusion.


<a id="org1120392"></a>

## TODO Create a custom table component in Apache superset     :REACT:UI:


<a id="org3a793b7"></a>

## TODO Explore web assembly for own UI


<a id="org3f6aa48"></a>

## TODO AI based analysis


<a id="org9617094"></a>

## TODO Self healing     :POC:


<a id="org21380fb"></a>

## TODO Edge analysis


<a id="org654cf2d"></a>

## TODO Analyse the gather already gathered and get some


<a id="org74a3ed4"></a>

## TODO 


<a id="orgef3dfec"></a>

# Decisions


<a id="org02a31ed"></a>

## WAL file system

We need a write ahead log based file system for 2 reasons.

-   Parquet files need to be created and pushed into iceberg schema. This must be batched to avoid small file problems.
-   Logs must be durable.


<a id="org94bdc2d"></a>

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


<a id="org8c136e0"></a>

## Challenges

Writes can only be sequential, and sequential writes may not be able to fully saturate the modern SSDs,
For parallelization, there are a few options.


<a id="orgf74e1e2"></a>

### Atomic offset calculations

We can calculate the total block required for a buffer and atomically store it. Next I/O request will get this offset and then add it&rsquo;s size
automically and save it.
However, the downside is that to calculate the size of `RecordBuffers` accurately, we need to copy the data over to a `vec![]` datastructure.
This means we have to create another copy of the data in memory, which may not be very effecient.


<a id="orgd43ab5a"></a>

### Multi-log files / dividing log files into chunks.

This approach of creating 4 log files of 1 Gb, instead of a single 4 Gb logfile, could allow the work to parallelize.
However, this approach may be more complex and needs more dilibrations. Recovery could become pretty complex.


<a id="orged2be23"></a>

## Considerations

-   [] Need support for vectorized search of block headers. We can recover in parallel and create parquets in parallel using rayon.
-   [] Need a way to parallelize writes.
-   [] Dataloss of a few seconds may be acceptable. (Flushing for every entry in WAL could be expensive, databases flush on commit, we can flush every 2 seconds maybe)
-   [] Check-sum and corruption checks must be done.
-   [] Metadata should be field extractable, and no incur full serialization. (Flatbuf allows for primitive types to be de-serialized without reading entire payload)
-   [] Need some paddings in the WAL buffer for SIMD accelerations.
-   [] Need some padding in the headers for future explansions.
-   [] Need schema evolution options for the Metadata
-   [] Need validation scripts for checking block health and checksums for troubleshootings.

