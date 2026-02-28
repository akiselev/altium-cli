# ryankurte-ATSAMD21G.SchLib

## Version
Original: Header:        Protel for Windows - Schematic Library Editor Binary File Version 5.0
Minor version: 2

## Save-As Result
Success

## CFB Diff
```
DIFF ERROR: Files differ: first difference at byte offset 0x0000001a (26)
  File sizes differ: 12288 bytes vs 28672 bytes
  storage OK: /ATSAMD21G
  stream DIFFERS: /ATSAMD21G/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /ATSAMD21G/Data: text(259 bytes) vs text(259 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=ATSAMD21G|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=NNKGLQXI|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|ALLPINCOUNT=48
      file2 text: |RECORD=1|LibReference=ATSAMD21G|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=NNKGLQXI|AreaColor=11599871|Color=128|PartIDLocked=T|AllPinCount=48
    block[1] differs in /ATSAMD21G/Data: text(137 bytes) vs text(137 bytes)
      file1 text: |RECORD=14|ISNOTACCESIBLE=T|OWNERPARTID=1|LOCATION.X=-60|LOCATION.Y=-160|CORNER.X=60|CORNER.Y=150|COLOR=128|AREACOLOR=11599871|ISSOLID=T
      file2 text: |RECORD=14|IsNotAccesible=T|OwnerPartId=1|Location.X=-60|Location.Y=-160|Corner.X=60|Corner.Y=150|Color=128|AreaColor=11599871|IsSolid=T
    block[50] differs in /ATSAMD21G/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=U?|NAME=Designator|READONLYSTATE=1|UNIQUEID=KYMNACYW
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=U?|Name=Designator|ReadOnlyState=1|UniqueID=KYMNACYW
    block[51] differs in /ATSAMD21G/Data: text(132 bytes) vs text(132 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|TEXT=*|NAME=Comment|UNIQUEID=ADDIMBUS
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|Text=*|Name=Comment|UniqueID=ADDIMBUS
  stream DIFFERS: /ATSAMD21G/PinPackageLength
    first diff at stream offset 0x0000001e (30)
    file1 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 45 49 47 48 54 3d 34 38 
    file2 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 65 69 67 68 74 3d 34 38 
    block[0] differs in /ATSAMD21G/PinPackageLength: text(35 bytes) vs text(35 bytes)
      file1 text: |HEADER=PinPackageLength|WEIGHT=48
      file2 text: |HEADER=PinPackageLength|Weight=48
  stream DIFFERS: /ATSAMD21G/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 34 38 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 34 38 
    block[0] differs in /ATSAMD21G/PinSymbolLineWidth: text(37 bytes) vs text(37 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=48
      file2 text: |HEADER=PinSymbolLineWidth|Weight=48
  stream DIFFERS: /FileHeader
    first diff at stream offset 0x00000053 (83)
    file1 hex [0x0000004b+]: 6f 6e 20 35 2e 30 7c 57 45 49 47 48 54 3d 35 34 
    file2 hex [0x0000004b+]: 6f 6e 20 35 2e 30 7c 57 65 69 67 68 74 3d 35 34 
    block[0] differs in /FileHeader: text(441 bytes) vs text(441 bytes)
      file1 text: |HEADER=Protel for Windows - Schematic Library Editor Binary File Version 5.0|WEIGHT=54|MINORVERSION=2|UNIQUEID=HHHPGTYL|FONTIDCOUNT=1|SIZE1=10|FONTNAME1=Times New Roman|USEMBCS=T|ISBOC=T|SHEETSTYLE=9|BORDERON=T|SHEETNUMBERSPACESIZE=12|AREACOLOR=16317695|SNAPGRIDON=T|SNAPGRIDSIZE=10|VISIBLEGRIDON=T|VISIBLEGRIDSIZE=10|CUSTOMX=18000|CUSTOMY=18000|USECUSTOMSHEET=T|REFERENCEZONESON=T|DISPLAY_UNIT=0|COMPCOUNT=1|LIBREF0=ATSAMD21G|PARTCOUNT0=2
      file2 text: |HEADER=Protel for Windows - Schematic Library Editor Binary File Version 5.0|Weight=54|MinorVersion=2|UniqueID=HHHPGTYL|FontIdCount=1|Size1=10|FontName1=Times New Roman|UseMBCS=T|IsBOC=T|SheetStyle=9|BorderOn=T|SheetNumberSpaceSize=12|AreaColor=16317695|SnapGridOn=T|SnapGridSize=10|VisibleGridOn=T|VisibleGridSize=10|CustomX=18000|CustomY=18000|UseCustomSheet=T|ReferenceZonesOn=T|Display_Unit=0|CompCount=1|LibRef0=ATSAMD21G|PartCount0=2
  stream OK: /Storage (25 bytes)
```
