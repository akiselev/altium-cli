# LEDs.SchLib

## Version
Original: Header:        Protel for Windows - Schematic Library Editor Binary File Version 5.0
Minor version: 2

## Save-As Result
Success
```
Saved: /home/kiselev/git/altium-cli-simplified/data/schlib/LEDs.SchLib -> /home/kiselev/git/altium-cli-simplified/data/schlib-saveas/LEDs.SchLib
```

## CFB Diff
```
DIFF ERROR:
Files differ: first difference at byte offset 0x0000001a (26)
  File sizes differ: 82944 bytes vs 94208 bytes
  storage OK: /APA102
  stream DIFFERS: /APA102/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /APA102/Data: text(241 bytes) vs text(241 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=APA102|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=FOJDEBPF|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T
      file2 text: |RECORD=1|LibReference=APA102|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=FOJDEBPF|AreaColor=11599871|Color=128|PartIDLocked=T
    block[1] differs in /APA102/Data: text(135 bytes) vs text(135 bytes)
      file1 text: |RECORD=14|ISNOTACCESIBLE=T|OWNERPARTID=1|LOCATION.X=-30|LOCATION.Y=-20|CORNER.X=30|CORNER.Y=20|COLOR=128|AREACOLOR=11599871|ISSOLID=T
      file2 text: |RECORD=14|IsNotAccesible=T|OwnerPartId=1|Location.X=-30|Location.Y=-20|Corner.X=30|Corner.Y=20|Color=128|AreaColor=11599871|IsSolid=T
    block[8] differs in /APA102/Data: text(149 bytes) vs text(149 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=*|NAME=Designator|READONLYSTATE=1|UNIQUEID=MCDUQMRT
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=*|Name=Designator|ReadOnlyState=1|UniqueID=MCDUQMRT
    block[9] differs in /APA102/Data: text(132 bytes) vs text(132 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|TEXT=*|NAME=Comment|UNIQUEID=MOPTPFFJ
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|Text=*|Name=Comment|UniqueID=MOPTPFFJ
    block[11] differs in /APA102/Data: text(189 bytes) vs text(189 bytes)
      file1 text: |RECORD=45|OWNERINDEX=10|INDEXINSHEET=-1|MODELNAME=LED SMD 5x5mm|MODELTYPE=PCBLIB|DATAFILECOUNT=1|MODELDATAFILEENTITY0=LED SMD 5x5mm|MODELDATAFILEKIND0=PCBLib|ISCURRENT=T|UNIQUEID=LUFBOSEK
      file2 text: |RECORD=45|OwnerIndex=10|IndexInSheet=-1|ModelName=LED SMD 5x5mm|ModelType=PCBLIB|DatafileCount=1|ModelDatafileEntity0=LED SMD 5x5mm|ModelDatafileKind0=PCBLib|IsCurrent=T|UniqueID=LUFBOSEK
    block[12] differs in /APA102/Data: text(25 bytes) vs text(25 bytes)
      file1 text: |RECORD=46|OWNERINDEX=11
      file2 text: |RECORD=46|OwnerIndex=11
    block[13] differs in /APA102/Data: text(25 bytes) vs text(25 bytes)
      file1 text: |RECORD=48|OWNERINDEX=11
      file2 text: |RECORD=48|OwnerIndex=11
  stream DIFFERS: /APA102/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 36 00 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 36 00 
    block[0] differs in /APA102/PinSymbolLineWidth: text(36 bytes) vs text(36 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=6
      file2 text: |HEADER=PinSymbolLineWidth|Weight=6
    block[1] differs in /APA102/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /APA102/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[3] differs in /APA102/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[4] differs in /APA102/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[5] differs in /APA102/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[6] differs in /APA102/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
  stream DIFFERS: /FileHeader
    first diff at stream offset 0x00000053 (83)
    file1 hex [0x0000004b+]: 6f 6e 20 35 2e 30 7c 57 45 49 47 48 54 3d 31 31 
    file2 hex [0x0000004b+]: 6f 6e 20 35 2e 30 7c 57 65 69 67 68 74 3d 31 31 
    block[0] differs in /FileHeader: text(841 bytes) vs text(841 bytes)
      file1 text: |HEADER=Protel for Windows - Schematic Library Editor Binary File Version 5.0|WEIGHT=119|MINORVERSION=2|UNIQUEID=BHHJMVAL|FONTIDCOUNT=2|SIZE1=10|FONTNAME1=Times New Roman|SIZE2=10|UNDERLINE2=T|FONTNAME2=Times New Roman|USEMBCS=T|ISBOC=T|SHEETSTYLE=9|BORDERON=T|SHEETNUMBERSPACESIZE=4|AREACOLOR=16317695|SNAPGRIDON=T|SNAPGRIDSIZE=10|VISIBLEGRIDON=T|VISIBLEGRIDSIZE=10|CUSTOMX=2000|CUSTOMY=2000|USECUSTOMSHEET=T|REFERENCEZONESON=T|DISPLAY_UNIT=4|COMPCOUNT=9|LIBREF0=Vishay VDMY10A1|COMPDESCR0=7-segment display, SMD, yellow|PARTCOUNT0=2|LIBREF1=LED strip single color|PARTCOUNT1=2|LIBREF2=LED chip RGB 100W|PARTCOUNT2=2|LIBREF3=LED strip WS2812|PARTCOUNT3=2|LIBREF4=LED chip RGB 30W|PARTCOUNT4=2|LIBREF5=LED strip RGB|PARTCOUNT5=2|LIBREF6=WS2812|COMPDESCR6=Addressable RGB LED|PARTCOUNT6=2|LIBREF7=APA102|PARTCOUNT7=2|LIBREF8=LED|PARTCOUNT8=2
      file2 text: |HEADER=Protel for Windows - Schematic Library Editor Binary File Version 5.0|Weight=119|MinorVersion=2|UniqueID=BHHJMVAL|FontIdCount=2|Size1=10|FontName1=Times New Roman|Size2=10|Underline2=T|FontName2=Times New Roman|UseMBCS=T|IsBOC=T|SheetStyle=9|BorderOn=T|SheetNumberSpaceSize=4|AreaColor=16317695|SnapGridOn=T|SnapGridSize=10|VisibleGridOn=T|VisibleGridSize=10|CustomX=2000|CustomY=2000|UseCustomSheet=T|ReferenceZonesOn=T|Display_Unit=4|CompCount=9|LibRef0=Vishay VDMY10A1|CompDescr0=7-segment display, SMD, yellow|PartCount0=2|LibRef1=LED strip single color|PartCount1=2|LibRef2=LED chip RGB 100W|PartCount2=2|LibRef3=LED strip WS2812|PartCount3=2|LibRef4=LED chip RGB 30W|PartCount4=2|LibRef5=LED strip RGB|PartCount5=2|LibRef6=WS2812|CompDescr6=Addressable RGB LED|PartCount6=2|LibRef7=APA102|PartCount7=2|LibRef8=LED|PartCount8=2
  storage OK: /LED
  storage OK: /LED chip RGB 100W
  stream DIFFERS: /LED chip RGB 100W/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /LED chip RGB 100W/Data: text(283 bytes) vs text(283 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=LED chip RGB 100W|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=PDPMXBLQ|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=LED chip RGB 100W
      file2 text: |RECORD=1|LibReference=LED chip RGB 100W|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=PDPMXBLQ|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=LED chip RGB 100W
    block[1] differs in /LED chip RGB 100W/Data: text(135 bytes) vs text(135 bytes)
      file1 text: |RECORD=14|ISNOTACCESIBLE=T|OWNERPARTID=1|LOCATION.X=-20|LOCATION.Y=-20|CORNER.X=20|CORNER.Y=20|COLOR=128|AREACOLOR=11599871|ISSOLID=T
      file2 text: |RECORD=14|IsNotAccesible=T|OwnerPartId=1|Location.X=-20|Location.Y=-20|Corner.X=20|Corner.Y=20|Color=128|AreaColor=11599871|IsSolid=T
    block[8] differs in /LED chip RGB 100W/Data: text(149 bytes) vs text(149 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=*|NAME=Designator|READONLYSTATE=1|UNIQUEID=AFKTAWKX
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=*|Name=Designator|ReadOnlyState=1|UniqueID=AFKTAWKX
    block[9] differs in /LED chip RGB 100W/Data: text(132 bytes) vs text(132 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|TEXT=*|NAME=Comment|UNIQUEID=LROHBXCW
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|Text=*|Name=Comment|UniqueID=LROHBXCW
    block[11] differs in /LED chip RGB 100W/Data: text(211 bytes) vs text(211 bytes)
      file1 text: |RECORD=45|OWNERINDEX=10|INDEXINSHEET=-1|MODELNAME=LED Chip RGB 100W Cutout|MODELTYPE=PCBLIB|DATAFILECOUNT=1|MODELDATAFILEENTITY0=LED Chip RGB 100W Cutout|MODELDATAFILEKIND0=PCBLib|ISCURRENT=T|UNIQUEID=RXNGNRDV
      file2 text: |RECORD=45|OwnerIndex=10|IndexInSheet=-1|ModelName=LED Chip RGB 100W Cutout|ModelType=PCBLIB|DatafileCount=1|ModelDatafileEntity0=LED Chip RGB 100W Cutout|ModelDatafileKind0=PCBLib|IsCurrent=T|UniqueID=RXNGNRDV
    block[12] differs in /LED chip RGB 100W/Data: text(25 bytes) vs text(25 bytes)
      file1 text: |RECORD=46|OWNERINDEX=11
      file2 text: |RECORD=46|OwnerIndex=11
    block[13] differs in /LED chip RGB 100W/Data: text(25 bytes) vs text(25 bytes)
      file1 text: |RECORD=48|OWNERINDEX=11
      file2 text: |RECORD=48|OwnerIndex=11
  stream DIFFERS: /LED chip RGB 100W/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 36 00 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 36 00 
    block[0] differs in /LED chip RGB 100W/PinSymbolLineWidth: text(36 bytes) vs text(36 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=6
      file2 text: |HEADER=PinSymbolLineWidth|Weight=6
    block[1] differs in /LED chip RGB 100W/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /LED chip RGB 100W/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[3] differs in /LED chip RGB 100W/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[4] differs in /LED chip RGB 100W/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[5] differs in /LED chip RGB 100W/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[6] differs in /LED chip RGB 100W/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
  storage OK: /LED chip RGB 30W
  stream DIFFERS: /LED chip RGB 30W/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /LED chip RGB 30W/Data: text(281 bytes) vs text(281 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=LED chip RGB 30W|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=ACFHVGTK|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=LED chip RGB 30W
      file2 text: |RECORD=1|LibReference=LED chip RGB 30W|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=ACFHVGTK|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=LED chip RGB 30W
    block[1] differs in /LED chip RGB 30W/Data: text(135 bytes) vs text(135 bytes)
      file1 text: |RECORD=14|ISNOTACCESIBLE=T|OWNERPARTID=1|LOCATION.X=-20|LOCATION.Y=-30|CORNER.X=20|CORNER.Y=20|COLOR=128|AREACOLOR=11599871|ISSOLID=T
      file2 text: |RECORD=14|IsNotAccesible=T|OwnerPartId=1|Location.X=-20|Location.Y=-30|Corner.X=20|Corner.Y=20|Color=128|AreaColor=11599871|IsSolid=T
    block[6] differs in /LED chip RGB 30W/Data: text(149 bytes) vs text(149 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=*|NAME=Designator|READONLYSTATE=1|UNIQUEID=XSIDRAEO
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=*|Name=Designator|ReadOnlyState=1|UniqueID=XSIDRAEO
    block[7] differs in /LED chip RGB 30W/Data: text(132 bytes) vs text(132 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|TEXT=*|NAME=Comment|UNIQUEID=GEKYHHPI
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|Text=*|Name=Comment|UniqueID=GEKYHHPI
  stream DIFFERS: /LED chip RGB 30W/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 34 00 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 34 00 
    block[0] differs in /LED chip RGB 30W/PinSymbolLineWidth: text(36 bytes) vs text(36 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=4
      file2 text: |HEADER=PinSymbolLineWidth|Weight=4
    block[1] differs in /LED chip RGB 30W/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /LED chip RGB 30W/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[3] differs in /LED chip RGB 30W/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[4] differs in /LED chip RGB 30W/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
  storage OK: /LED strip RGB
  stream DIFFERS: /LED strip RGB/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /LED strip RGB/Data: text(275 bytes) vs text(275 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=LED strip RGB|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=NQMEYMWL|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=LED strip RGB
      file2 text: |RECORD=1|LibReference=LED strip RGB|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=NQMEYMWL|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=LED strip RGB
    block[1] differs in /LED strip RGB/Data: text(135 bytes) vs text(135 bytes)
      file1 text: |RECORD=14|ISNOTACCESIBLE=T|OWNERPARTID=1|LOCATION.X=-20|LOCATION.Y=-30|CORNER.X=10|CORNER.Y=20|COLOR=128|AREACOLOR=11599871|ISSOLID=T
      file2 text: |RECORD=14|IsNotAccesible=T|OwnerPartId=1|Location.X=-20|Location.Y=-30|Corner.X=10|Corner.Y=20|Color=128|AreaColor=11599871|IsSolid=T
    block[6] differs in /LED strip RGB/Data: text(149 bytes) vs text(149 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=*|NAME=Designator|READONLYSTATE=1|UNIQUEID=RLXEKIYP
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=*|Name=Designator|ReadOnlyState=1|UniqueID=RLXEKIYP
    block[7] differs in /LED strip RGB/Data: text(132 bytes) vs text(132 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|TEXT=*|NAME=Comment|UNIQUEID=AOWOBJXD
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|Text=*|Name=Comment|UniqueID=AOWOBJXD
    block[9] differs in /LED strip RGB/Data: text(194 bytes) vs text(194 bytes)
      file1 text: |RECORD=45|OWNERINDEX=8|INDEXINSHEET=-1|MODELNAME=LED strip 4 pads|MODELTYPE=PCBLIB|DATAFILECOUNT=1|MODELDATAFILEENTITY0=LED strip 4 pads|MODELDATAFILEKIND0=PCBLib|ISCURRENT=T|UNIQUEID=FGRWIHKY
      file2 text: |RECORD=45|OwnerIndex=8|IndexInSheet=-1|ModelName=LED strip 4 pads|ModelType=PCBLIB|DatafileCount=1|ModelDatafileEntity0=LED strip 4 pads|ModelDatafileKind0=PCBLib|IsCurrent=T|UniqueID=FGRWIHKY
    block[10] differs in /LED strip RGB/Data: text(24 bytes) vs text(24 bytes)
      file1 text: |RECORD=46|OWNERINDEX=9
      file2 text: |RECORD=46|OwnerIndex=9
    block[11] differs in /LED strip RGB/Data: text(24 bytes) vs text(24 bytes)
      file1 text: |RECORD=48|OWNERINDEX=9
      file2 text: |RECORD=48|OwnerIndex=9
  stream DIFFERS: /LED strip RGB/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 34 00 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 34 00 
    block[0] differs in /LED strip RGB/PinSymbolLineWidth: text(36 bytes) vs text(36 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=4
      file2 text: |HEADER=PinSymbolLineWidth|Weight=4
    block[1] differs in /LED strip RGB/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /LED strip RGB/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[3] differs in /LED strip RGB/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[4] differs in /LED strip RGB/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
  storage OK: /LED strip WS2812
  stream DIFFERS: /LED strip WS2812/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /LED strip WS2812/Data: text(281 bytes) vs text(281 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=LED strip WS2812|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=UVTYBGRT|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=LED strip WS2812
      file2 text: |RECORD=1|LibReference=LED strip WS2812|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=UVTYBGRT|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=LED strip WS2812
    block[1] differs in /LED strip WS2812/Data: text(120 bytes) vs text(120 bytes)
      file1 text: |RECORD=14|ISNOTACCESIBLE=T|OWNERPARTID=1|LOCATION.Y=-20|CORNER.X=40|CORNER.Y=20|COLOR=128|AREACOLOR=11599871|ISSOLID=T
      file2 text: |RECORD=14|IsNotAccesible=T|OwnerPartId=1|Location.Y=-20|Corner.X=40|Corner.Y=20|Color=128|AreaColor=11599871|IsSolid=T
    block[5] differs in /LED strip WS2812/Data: text(149 bytes) vs text(149 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=*|NAME=Designator|READONLYSTATE=1|UNIQUEID=XKOLXYMW
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=*|Name=Designator|ReadOnlyState=1|UniqueID=XKOLXYMW
    block[6] differs in /LED strip WS2812/Data: text(132 bytes) vs text(132 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|TEXT=*|NAME=Comment|UNIQUEID=WULYYBHW
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|Text=*|Name=Comment|UniqueID=WULYYBHW
    block[8] differs in /LED strip WS2812/Data: text(194 bytes) vs text(194 bytes)
      file1 text: |RECORD=45|OWNERINDEX=7|INDEXINSHEET=-1|MODELNAME=LED strip 3 pads|MODELTYPE=PCBLIB|DATAFILECOUNT=1|MODELDATAFILEENTITY0=LED strip 3 pads|MODELDATAFILEKIND0=PCBLib|ISCURRENT=T|UNIQUEID=MKBSQECF
      file2 text: |RECORD=45|OwnerIndex=7|IndexInSheet=-1|ModelName=LED strip 3 pads|ModelType=PCBLIB|DatafileCount=1|ModelDatafileEntity0=LED strip 3 pads|ModelDatafileKind0=PCBLib|IsCurrent=T|UniqueID=MKBSQECF
    block[9] differs in /LED strip WS2812/Data: text(24 bytes) vs text(24 bytes)
      file1 text: |RECORD=46|OWNERINDEX=8
      file2 text: |RECORD=46|OwnerIndex=8
    block[10] differs in /LED strip WS2812/Data: text(24 bytes) vs text(24 bytes)
      file1 text: |RECORD=48|OWNERINDEX=8
      file2 text: |RECORD=48|OwnerIndex=8
  stream DIFFERS: /LED strip WS2812/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 33 00 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 33 00 
    block[0] differs in /LED strip WS2812/PinSymbolLineWidth: text(36 bytes) vs text(36 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=3
      file2 text: |HEADER=PinSymbolLineWidth|Weight=3
    block[1] differs in /LED strip WS2812/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /LED strip WS2812/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[3] differs in /LED strip WS2812/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
  storage OK: /LED strip single color
  stream DIFFERS: /LED strip single color/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /LED strip single color/Data: text(293 bytes) vs text(293 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=LED strip single color|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=NJBXCMHI|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=LED strip single color
      file2 text: |RECORD=1|LibReference=LED strip single color|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=NJBXCMHI|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=LED strip single color
    block[1] differs in /LED strip single color/Data: text(135 bytes) vs text(135 bytes)
      file1 text: |RECORD=14|ISNOTACCESIBLE=T|OWNERPARTID=1|LOCATION.X=-10|LOCATION.Y=-10|CORNER.X=30|CORNER.Y=20|COLOR=128|AREACOLOR=11599871|ISSOLID=T
      file2 text: |RECORD=14|IsNotAccesible=T|OwnerPartId=1|Location.X=-10|Location.Y=-10|Corner.X=30|Corner.Y=20|Color=128|AreaColor=11599871|IsSolid=T
    block[4] differs in /LED strip single color/Data: text(149 bytes) vs text(149 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=*|NAME=Designator|READONLYSTATE=1|UNIQUEID=GTMKBONH
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=*|Name=Designator|ReadOnlyState=1|UniqueID=GTMKBONH
    block[5] differs in /LED strip single color/Data: text(132 bytes) vs text(132 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|TEXT=*|NAME=Comment|UNIQUEID=VYASOCNT
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|Text=*|Name=Comment|UniqueID=VYASOCNT
    block[7] differs in /LED strip single color/Data: text(194 bytes) vs text(194 bytes)
      file1 text: |RECORD=45|OWNERINDEX=6|INDEXINSHEET=-1|MODELNAME=LED strip 2 pads|MODELTYPE=PCBLIB|DATAFILECOUNT=1|MODELDATAFILEENTITY0=LED strip 2 pads|MODELDATAFILEKIND0=PCBLib|ISCURRENT=T|UNIQUEID=XTCWXGLR
      file2 text: |RECORD=45|OwnerIndex=6|IndexInSheet=-1|ModelName=LED strip 2 pads|ModelType=PCBLIB|DatafileCount=1|ModelDatafileEntity0=LED strip 2 pads|ModelDatafileKind0=PCBLib|IsCurrent=T|UniqueID=XTCWXGLR
    block[8] differs in /LED strip single color/Data: text(24 bytes) vs text(24 bytes)
      file1 text: |RECORD=46|OWNERINDEX=7
      file2 text: |RECORD=46|OwnerIndex=7
    block[9] differs in /LED strip single color/Data: text(24 bytes) vs text(24 bytes)
      file1 text: |RECORD=48|OWNERINDEX=7
      file2 text: |RECORD=48|OwnerIndex=7
  stream DIFFERS: /LED strip single color/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 32 00 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 32 00 
    block[0] differs in /LED strip single color/PinSymbolLineWidth: text(36 bytes) vs text(36 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=2
      file2 text: |HEADER=PinSymbolLineWidth|Weight=2
    block[1] differs in /LED strip single color/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /LED strip single color/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
  stream DIFFERS: /LED/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /LED/Data: text(238 bytes) vs text(238 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=LED|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=XPFNSKUQ|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T
      file2 text: |RECORD=1|LibReference=LED|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=XPFNSKUQ|AreaColor=11599871|Color=128|PartIDLocked=T
    block[3] differs in /LED/Data: text(131 bytes) vs text(131 bytes)
      file1 text: |RECORD=6|ISNOTACCESIBLE=T|INDEXINSHEET=2|OWNERPARTID=1|LINEWIDTH=1|LOCATIONCOUNT=4|X1=-20|Y1=10|X2=-20|Y2=-10|X3=-10|X4=-20|Y4=10
      file2 text: |RECORD=6|IsNotAccesible=T|IndexInSheet=2|OwnerPartId=1|LineWidth=1|LocationCount=4|X1=-20|Y1=10|X2=-20|Y2=-10|X3=-10|X4=-20|Y4=10
    block[4] differs in /LED/Data: text(141 bytes) vs text(141 bytes)
      file1 text: |RECORD=6|ISNOTACCESIBLE=T|INDEXINSHEET=3|OWNERPARTID=1|LINEWIDTH=1|LOCATIONCOUNT=5|X1=-7|Y1=12|X2=3|Y2=32|X3=3|Y3=22|X4=3|Y4=32|X5=-5|Y5=27
      file2 text: |RECORD=6|IsNotAccesible=T|IndexInSheet=3|OwnerPartId=1|LineWidth=1|LocationCount=5|X1=-7|Y1=12|X2=3|Y2=32|X3=3|Y3=22|X4=3|Y4=32|X5=-5|Y5=27
    block[5] differs in /LED/Data: text(146 bytes) vs text(146 bytes)
      file1 text: |RECORD=6|ISNOTACCESIBLE=T|INDEXINSHEET=4|OWNERPARTID=1|LINEWIDTH=1|LOCATIONCOUNT=5|X1=-17|Y1=15|X2=-7|Y2=35|X3=-7|Y3=25|X4=-7|Y4=35|X5=-15|Y5=30
      file2 text: |RECORD=6|IsNotAccesible=T|IndexInSheet=4|OwnerPartId=1|LineWidth=1|LocationCount=5|X1=-17|Y1=15|X2=-7|Y2=35|X3=-7|Y3=25|X4=-7|Y4=35|X5=-15|Y5=30
    block[6] differs in /LED/Data: text(111 bytes) vs text(111 bytes)
      file1 text: |RECORD=6|ISNOTACCESIBLE=T|INDEXINSHEET=5|OWNERPARTID=1|LINEWIDTH=1|LOCATIONCOUNT=2|X1=-10|Y1=-10|X2=-10|Y2=10
      file2 text: |RECORD=6|IsNotAccesible=T|IndexInSheet=5|OwnerPartId=1|LineWidth=1|LocationCount=2|X1=-10|Y1=-10|X2=-10|Y2=10
    block[7] differs in /LED/Data: text(98 bytes) vs text(98 bytes)
      file1 text: |RECORD=6|ISNOTACCESIBLE=T|INDEXINSHEET=6|OWNERPARTID=1|LINEWIDTH=1|LOCATIONCOUNT=2|X1=-10|X2=-20
      file2 text: |RECORD=6|IsNotAccesible=T|IndexInSheet=6|OwnerPartId=1|LineWidth=1|LocationCount=2|X1=-10|X2=-20
    block[8] differs in /LED/Data: text(149 bytes) vs text(149 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=*|NAME=Designator|READONLYSTATE=1|UNIQUEID=VOAMLVYU
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=*|Name=Designator|ReadOnlyState=1|UniqueID=VOAMLVYU
    block[9] differs in /LED/Data: text(132 bytes) vs text(132 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|TEXT=*|NAME=Comment|UNIQUEID=MUMVESMA
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|Text=*|Name=Comment|UniqueID=MUMVESMA
  stream DIFFERS: /LED/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 32 00 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 32 00 
    block[0] differs in /LED/PinSymbolLineWidth: text(36 bytes) vs text(36 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=2
      file2 text: |HEADER=PinSymbolLineWidth|Weight=2
    block[1] differs in /LED/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /LED/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
  stream DIFFERS: /Storage
    length: 59568 bytes vs 61154 bytes
    first diff at stream offset 0x0000001a (26)
    file1 hex [0x00000012+]: 74 6f 72 61 67 65 7c 57 45 49 47 48 54 3d 31 00 
    file2 hex [0x00000012+]: 74 6f 72 61 67 65 7c 57 65 69 67 68 74 3d 31 00 
    block[0] differs in /Storage: text(30 bytes) vs text(30 bytes)
      file1 text: |HEADER=Icon storage|WEIGHT=1
      file2 text: |HEADER=Icon storage|Weight=1
    block[1] differs in /Storage: binary(59530 bytes) vs binary(61116 bytes)
  storage OK: /Vishay VDMY10A1
  stream DIFFERS: /Vishay VDMY10A1/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /Vishay VDMY10A1/Data: text(331 bytes) vs text(331 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=Vishay VDMY10A1|COMPONENTDESCRIPTION=7-segment display, SMD, yellow|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=KPJGNLDX|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=Vishay VDMY10A1
      file2 text: |RECORD=1|LibReference=Vishay VDMY10A1|ComponentDescription=7-segment display, SMD, yellow|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=KPJGNLDX|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=Vishay VDMY10A1
    block[1] differs in /Vishay VDMY10A1/Data: text(134 bytes) vs text(134 bytes)
      file1 text: |RECORD=41|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-25|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DigiKey|NAME=Supplier|UNIQUEID=WLYALDAF
      file2 text: |RECORD=41|OwnerPartId=-1|Location.X=-5|Location.Y=-25|Color=8388608|FontID=1|IsHidden=T|Text=DigiKey|Name=Supplier|UniqueID=WLYALDAF
    block[2] differs in /Vishay VDMY10A1/Data: text(152 bytes) vs text(152 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-25|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=Vishay|NAME=Manufacturer|UNIQUEID=BJEQHONU
      file2 text: |RECORD=41|IndexInSheet=1|OwnerPartId=-1|Location.X=-5|Location.Y=-25|Color=8388608|FontID=1|IsHidden=T|Text=Vishay|Name=Manufacturer|UniqueID=BJEQHONU
    block[3] differs in /Vishay VDMY10A1/Data: text(163 bytes) vs text(163 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=2|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-25|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=VDMY10A1|NAME=Manufacturer Part Nr.|UNIQUEID=GNHWLALA
      file2 text: |RECORD=41|IndexInSheet=2|OwnerPartId=-1|Location.X=-5|Location.Y=-25|Color=8388608|FontID=1|IsHidden=T|Text=VDMY10A1|Name=Manufacturer Part Nr.|UniqueID=GNHWLALA
    block[4] differs in /Vishay VDMY10A1/Data: text(249 bytes) vs text(249 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=3|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-25|COLOR=8388608|FONTID=2|ISHIDDEN=T|TEXT=https://www.digikey.de/product-detail/de/vishay-semiconductor-opto-division/VDMY10A1/VDMY10A1CT-ND/|NAME=DigiKey Part URL|UNIQUEID=UTWQVVVF
      file2 text: |RECORD=41|IndexInSheet=3|OwnerPartId=-1|Location.X=-5|Location.Y=-25|Color=8388608|FontID=2|IsHidden=T|Text=https://www.digikey.de/product-detail/de/vishay-semiconductor-opto-division/VDMY10A1/VDMY10A1CT-ND/|Name=DigiKey Part URL|UniqueID=UTWQVVVF
    block[5] differs in /Vishay VDMY10A1/Data: text(163 bytes) vs text(163 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=4|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-25|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=VDMY10A1CT-ND|NAME=DigiKey Part Nr.|UNIQUEID=FALVYQTY
      file2 text: |RECORD=41|IndexInSheet=4|OwnerPartId=-1|Location.X=-5|Location.Y=-25|Color=8388608|FontID=1|IsHidden=T|Text=VDMY10A1CT-ND|Name=DigiKey Part Nr.|UniqueID=FALVYQTY
    block[6] differs in /Vishay VDMY10A1/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=14|ISNOTACCESIBLE=T|INDEXINSHEET=5|OWNERPARTID=1|LOCATION.X=-70|LOCATION.Y=-60|CORNER.X=80|CORNER.Y=50|COLOR=128|AREACOLOR=11599871|ISSOLID=T
      file2 text: |RECORD=14|IsNotAccesible=T|IndexInSheet=5|OwnerPartId=1|Location.X=-70|Location.Y=-60|Corner.X=80|Corner.Y=50|Color=128|AreaColor=11599871|IsSolid=T
    block[17] differs in /Vishay VDMY10A1/Data: text(225 bytes) vs text(225 bytes)
      file1 text: |RECORD=30|ISNOTACCESIBLE=T|INDEXINSHEET=16|OWNERPARTID=1|GRAPHICALLYLOCKED=T|LOCATION.X=-44|LOCATION.Y=-55|CORNER.X=39|CORNER.X_FRAC=33333|CORNER.Y=45|KEEPASPECT=T|EMBEDIMAGE=T|FILENAME=C:\Users\mbock\Downloads\vdmx10a1.png
      file2 text: |RECORD=30|IsNotAccesible=T|IndexInSheet=16|OwnerPartId=1|GraphicallyLocked=T|Location.X=-44|Location.Y=-55|Corner.X=39|Corner.X_Frac=33333|Corner.Y=45|KeepAspect=T|EmbedImage=T|FileName=C:\Users\mbock\Downloads\vdmx10a1.png
    block[18] differs in /Vishay VDMY10A1/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=D?|NAME=Designator|READONLYSTATE=1|UNIQUEID=DNQEJLRS
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=D?|Name=Designator|ReadOnlyState=1|UniqueID=DNQEJLRS
    block[19] differs in /Vishay VDMY10A1/Data: text(148 bytes) vs text(148 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|TEXT=7-segment display|NAME=Comment|UNIQUEID=JPUHDSJW
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|Text=7-segment display|Name=Comment|UniqueID=JPUHDSJW
    block[21] differs in /Vishay VDMY10A1/Data: text(193 bytes) vs text(193 bytes)
      file1 text: |RECORD=45|OWNERINDEX=20|INDEXINSHEET=-1|MODELNAME=Vishay VDMx10A1|MODELTYPE=PCBLIB|DATAFILECOUNT=1|MODELDATAFILEENTITY0=Vishay VDMx10A1|MODELDATAFILEKIND0=PCBLib|ISCURRENT=T|UNIQUEID=WCPOEIJB
      file2 text: |RECORD=45|OwnerIndex=20|IndexInSheet=-1|ModelName=Vishay VDMx10A1|ModelType=PCBLIB|DatafileCount=1|ModelDatafileEntity0=Vishay VDMx10A1|ModelDatafileKind0=PCBLib|IsCurrent=T|UniqueID=WCPOEIJB
    block[22] differs in /Vishay VDMY10A1/Data: text(25 bytes) vs text(25 bytes)
      file1 text: |RECORD=46|OWNERINDEX=21
      file2 text: |RECORD=46|OwnerIndex=21
    block[23] differs in /Vishay VDMY10A1/Data: text(25 bytes) vs text(25 bytes)
      file1 text: |RECORD=48|OWNERINDEX=21
      file2 text: |RECORD=48|OwnerIndex=21
  stream DIFFERS: /Vishay VDMY10A1/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 31 30 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 31 30 
    block[0] differs in /Vishay VDMY10A1/PinSymbolLineWidth: text(37 bytes) vs text(37 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=10
      file2 text: |HEADER=PinSymbolLineWidth|Weight=10
    block[1] differs in /Vishay VDMY10A1/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /Vishay VDMY10A1/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[3] differs in /Vishay VDMY10A1/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[4] differs in /Vishay VDMY10A1/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[5] differs in /Vishay VDMY10A1/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[6] differs in /Vishay VDMY10A1/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[7] differs in /Vishay VDMY10A1/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[8] differs in /Vishay VDMY10A1/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[9] differs in /Vishay VDMY10A1/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[10] differs in /Vishay VDMY10A1/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
  storage OK: /WS2812
  stream DIFFERS: /WS2812/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /WS2812/Data: text(302 bytes) vs text(302 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=WS2812|COMPONENTDESCRIPTION=Addressable RGB LED|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=UKVOYNNR|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=WS2812
      file2 text: |RECORD=1|LibReference=WS2812|ComponentDescription=Addressable RGB LED|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=UKVOYNNR|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=WS2812
    block[1] differs in /WS2812/Data: text(223 bytes) vs text(223 bytes)
      file1 text: |RECORD=41|OWNERPARTID=-1|LOCATION.X=-72|LOCATION.Y=-92|COLOR=8388608|FONTID=2|ISHIDDEN=T|TEXT=https://www.digikey.de/product-detail/de/sparkfun-electronics/COM-11821/1568-1800-ND/6163706|NAME=DigiKey URL|UNIQUEID=VLGKBOWO
      file2 text: |RECORD=41|OwnerPartId=-1|Location.X=-72|Location.Y=-92|Color=8388608|FontID=2|IsHidden=T|Text=https://www.digikey.de/product-detail/de/sparkfun-electronics/COM-11821/1568-1800-ND/6163706|Name=DigiKey URL|UniqueID=VLGKBOWO
    block[2] differs in /WS2812/Data: text(166 bytes) vs text(166 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=-72|LOCATION.Y=-92|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=1568-1800-ND|NAME=DigiKey Part Number|UNIQUEID=WAPBHJOS
      file2 text: |RECORD=41|IndexInSheet=1|OwnerPartId=-1|Location.X=-72|Location.Y=-92|Color=8388608|FontID=1|IsHidden=T|Text=1568-1800-ND|Name=DigiKey Part Number|UniqueID=WAPBHJOS
    block[3] differs in /WS2812/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=14|ISNOTACCESIBLE=T|INDEXINSHEET=2|OWNERPARTID=1|LOCATION.X=-40|LOCATION.Y=-50|CORNER.X=40|CORNER.Y=40|COLOR=128|AREACOLOR=11599871|ISSOLID=T
      file2 text: |RECORD=14|IsNotAccesible=T|IndexInSheet=2|OwnerPartId=1|Location.X=-40|Location.Y=-50|Corner.X=40|Corner.Y=40|Color=128|AreaColor=11599871|IsSolid=T
    block[10] differs in /WS2812/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=D*|NAME=Designator|READONLYSTATE=1|UNIQUEID=MQUYSDXI
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=D*|Name=Designator|ReadOnlyState=1|UniqueID=MQUYSDXI
    block[11] differs in /WS2812/Data: text(138 bytes) vs text(138 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|TEXT=RGB LED|NAME=Comment|UNIQUEID=AFXCMMUG
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|Text=RGB LED|Name=Comment|UniqueID=AFXCMMUG
  stream DIFFERS: /WS2812/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 36 00 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 36 00 
    block[0] differs in /WS2812/PinSymbolLineWidth: text(36 bytes) vs text(36 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=6
      file2 text: |HEADER=PinSymbolLineWidth|Weight=6
    block[1] differs in /WS2812/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /WS2812/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[3] differs in /WS2812/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[4] differs in /WS2812/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[5] differs in /WS2812/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[6] differs in /WS2812/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
```
