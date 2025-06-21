
# Table of Contents

1.  [Understand the OTEL contract for logs](#orgbfa2720):SPIKE:
    1.  [Check how protobuf formats are translated into arrow columns](#org036fb82)
    2.  [Create a `build.rs` script with protobuf compilation and generations.](#org44ad261)
2.  [Unit test cases of configurations](#org5780f68):KUBER:
3.  [Create implementations of the Actix app server](#org1cc8a11)
    1.  [Injest server implementation](#org360967d)
        1.  [Create a tonic server](#orgc081460)
        2.  [Create a flight server](#org97e02e2)
    2.  [Query server implementation](#org77fde45)
        1.  [Create a health endpoint](#org72626e5)
        2.  [Create an add regex endpoint](#org2dd95c4)
4.  [Datafusion based SQL server](#org3c451be)
    1.  [datafusion-flight-sql-server](#orgac4ee45)
5.  [Flientbit agent](#orgdb34e72)
        1.  [Create a golang plugin](#org129ac50)
6.  [Create ebpf example in aya for XDP packet filtering](#orgcef99b2)
    1.  [Add support for maps to filter data](#org9ded5c2)
    2.  [Add support for actix web application.](#orgc7985f5)
7.  [Create a CI pipeline and build script](#org0690df0)
    1.  [Create a build.rs script for protobuf generations](#org406ef47)
    2.  [Create a actions script for building pipeline.](#orgccd9ef7)
    3.  [Create a make script](#org8661d62)
    4.  [Create a script for packaging into different distributions.](#org65ef338)
8.  [Actor models](#orgb1c4b67)
    1.  [Actor for managing regex patterns](#orgc4030f3)
    2.  [Actor for parsing json and converting into arrow](#org81ab31d)
        1.  [Explore **RAYON** for parallelism, we need not use actors because json parsing does not hold state.](#orgbc0efb0)
    3.  [Actor for sequential writing of data](#orgf01d59c)
    4.  [Actor for caching the parsed arrow buffers](#org27e0fdc)
    5.  [](#org83739f1)
9.  [Datafusion](#orgb5c0891)
    1.  [Query the data using datafusion.](#orgc5f23b4)
    2.  [Try to see of the query can be called from external clients.](#org3072472)
    3.  [Check if SQLAlchemy can be made to work with the Datafusion.](#org849028b)
10. [Create a custom table component in Apache superset](#org210198b):REACT:UI:
11. [Explore web assembly for own UI](#orgdce4864)
12. [AI based analysis](#orgaf79f88)
    1.  [Self healing](#org8f305f5):POC:
    2.  [Edge analysis](#orgd1c8381)
    3.  [Analyse the gather already gathered and get some](#orgb555ff7)
13. [](#org28a0c67)



<a id="orgbfa2720"></a>

# TODO Understand the OTEL contract for logs     :SPIKE:

We should strive to reuse these formats, we need interoperability and creating our own agents would prove too costly.


<a id="org036fb82"></a>

## TODO Check how protobuf formats are translated into arrow columns


<a id="org44ad261"></a>

## TODO Create a `build.rs` script with protobuf compilation and generations.


<a id="org5780f68"></a>

# TODO Unit test cases of configurations     :KUBER:


<a id="org1cc8a11"></a>

# TODO Create implementations of the Actix app server


<a id="org360967d"></a>

## TODO Injest server implementation


<a id="orgc081460"></a>

### TODO Create a tonic server


<a id="org97e02e2"></a>

### TODO Create a flight server


<a id="org77fde45"></a>

## TODO Query server implementation


<a id="org72626e5"></a>

### TODO Create a health endpoint


<a id="org2dd95c4"></a>

### TODO Create an add regex endpoint


<a id="org3c451be"></a>

# TODO Datafusion based SQL server


<a id="orgac4ee45"></a>

## TODO datafusion-flight-sql-server


<a id="orgdb34e72"></a>

# TODO Flientbit agent


<a id="org129ac50"></a>

### TODO Create a golang plugin

It should create arrow flight batches and ship it over grpc to collector

1.  TODO Create a skeleton plugin

2.  TODO Read and print the data

3.  TODO Create a contract for http, grpc traffic.

4.  TODO Create proxy interceptors in the contract.

5.  TODO Create a batching function

6.  TODO Side channel metadata - correlation - IP address, host id etc.


<a id="orgcef99b2"></a>

# TODO Create ebpf example in aya for XDP packet filtering


<a id="org9ded5c2"></a>

## TODO Add support for maps to filter data


<a id="orgc7985f5"></a>

## TODO Add support for actix web application.


<a id="org0690df0"></a>

# TODO Create a CI pipeline and build script


<a id="org406ef47"></a>

## TODO Create a build.rs script for protobuf generations


<a id="orgccd9ef7"></a>

## TODO Create a actions script for building pipeline.


<a id="org8661d62"></a>

## TODO Create a make script


<a id="org65ef338"></a>

## TODO Create a script for packaging into different distributions.


<a id="orgb1c4b67"></a>

# TODO Actor models


<a id="orgc4030f3"></a>

## TODO Actor for managing regex patterns


<a id="org81ab31d"></a>

## TODO Actor for parsing json and converting into arrow


<a id="orgbc0efb0"></a>

### TODO Explore **RAYON** for parallelism, we need not use actors because json parsing does not hold state.


<a id="orgf01d59c"></a>

## TODO Actor for sequential writing of data


<a id="org27e0fdc"></a>

## TODO Actor for caching the parsed arrow buffers


<a id="org83739f1"></a>

## TODO 


<a id="orgb5c0891"></a>

# TODO Datafusion


<a id="orgc5f23b4"></a>

## TODO Query the data using datafusion.


<a id="org3072472"></a>

## TODO Try to see of the query can be called from external clients.


<a id="org849028b"></a>

## TODO Check if SQLAlchemy can be made to work with the Datafusion.


<a id="org210198b"></a>

# TODO Create a custom table component in Apache superset     :REACT:UI:


<a id="orgdce4864"></a>

# TODO Explore web assembly for own UI


<a id="orgaf79f88"></a>

# TODO AI based analysis


<a id="org8f305f5"></a>

## TODO Self healing     :POC:


<a id="orgd1c8381"></a>

## TODO Edge analysis


<a id="orgb555ff7"></a>

## TODO Analyse the gather already gathered and get some


<a id="org28a0c67"></a>

# TODO 

