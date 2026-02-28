# Mechanical.SchLib

## Version
Original: Header:        Protel for Windows - Schematic Library Editor Binary File Version 5.0
Minor version: 2

## Save-As Result
Success
```
Saved: /home/kiselev/git/altium-cli-simplified/data/schlib/Mechanical.SchLib -> /home/kiselev/git/altium-cli-simplified/data/schlib-saveas/Mechanical.SchLib
```

## CFB Diff
```
DIFF ERROR:
Files differ: first difference at byte offset 0x0000001a (26)
  File sizes differ: 7168 bytes vs 20480 bytes
  stream DIFFERS: /FileHeader
    first diff at stream offset 0x00000053 (83)
    file1 hex [0x0000004b+]: 6f 6e 20 35 2e 30 7c 57 45 49 47 48 54 3d 32 39 
    file2 hex [0x0000004b+]: 6f 6e 20 35 2e 30 7c 57 65 69 67 68 74 3d 32 39 
    block[0] differs in /FileHeader: text(498 bytes) vs text(498 bytes)
      file1 text: |HEADER=Protel for Windows - Schematic Library Editor Binary File Version 5.0|WEIGHT=29|MINORVERSION=2|UNIQUEID=TNELJXPN|FONTIDCOUNT=1|SIZE1=10|FONTNAME1=Times New Roman|USEMBCS=T|ISBOC=T|SHEETSTYLE=9|BORDERON=T|SHEETNUMBERSPACESIZE=4|AREACOLOR=16317695|SNAPGRIDON=T|SNAPGRIDSIZE=10|VISIBLEGRIDON=T|VISIBLEGRIDSIZE=10|CUSTOMX=2000|CUSTOMY=2000|USECUSTOMSHEET=T|REFERENCEZONESON=T|DISPLAY_UNIT=4|COMPCOUNT=3|LIBREF0=Washer M6|PARTCOUNT0=2|LIBREF1=Screw M4|PARTCOUNT1=2|LIBREF2=Screw M3|PARTCOUNT2=2
      file2 text: |HEADER=Protel for Windows - Schematic Library Editor Binary File Version 5.0|Weight=29|MinorVersion=2|UniqueID=TNELJXPN|FontIdCount=1|Size1=10|FontName1=Times New Roman|UseMBCS=T|IsBOC=T|SheetStyle=9|BorderOn=T|SheetNumberSpaceSize=4|AreaColor=16317695|SnapGridOn=T|SnapGridSize=10|VisibleGridOn=T|VisibleGridSize=10|CustomX=2000|CustomY=2000|UseCustomSheet=T|ReferenceZonesOn=T|Display_Unit=4|CompCount=3|LibRef0=Washer M6|PartCount0=2|LibRef1=Screw M4|PartCount1=2|LibRef2=Screw M3|PartCount2=2
  storage OK: /Screw M3
  stream DIFFERS: /Screw M3/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /Screw M3/Data: text(265 bytes) vs text(265 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=Screw M3|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=JCITAKHO|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=Screw M3
      file2 text: |RECORD=1|LibReference=Screw M3|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=JCITAKHO|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=Screw M3
    block[1] differs in /Screw M3/Data: text(111 bytes) vs text(111 bytes)
      file1 text: |RECORD=8|ISNOTACCESIBLE=T|OWNERPARTID=1|RADIUS=10|SECONDARYRADIUS=10|LINEWIDTH=2|AREACOLOR=12632256|ISSOLID=T
      file2 text: |RECORD=8|IsNotAccesible=T|OwnerPartId=1|Radius=10|SecondaryRadius=10|LineWidth=2|AreaColor=12632256|IsSolid=T
    block[2] differs in /Screw M3/Data: text(97 bytes) vs text(97 bytes)
      file1 text: |RECORD=6|ISNOTACCESIBLE=T|INDEXINSHEET=1|OWNERPARTID=1|LINEWIDTH=1|LOCATIONCOUNT=2|Y1=10|Y2=-10
      file2 text: |RECORD=6|IsNotAccesible=T|IndexInSheet=1|OwnerPartId=1|LineWidth=1|LocationCount=2|Y1=10|Y2=-10
    block[3] differs in /Screw M3/Data: text(97 bytes) vs text(97 bytes)
      file1 text: |RECORD=6|ISNOTACCESIBLE=T|INDEXINSHEET=2|OWNERPARTID=1|LINEWIDTH=1|LOCATIONCOUNT=2|X1=-10|X2=10
      file2 text: |RECORD=6|IsNotAccesible=T|IndexInSheet=2|OwnerPartId=1|LineWidth=1|LocationCount=2|X1=-10|X2=10
    block[4] differs in /Screw M3/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=S1|NAME=Designator|READONLYSTATE=1|UNIQUEID=NKHMQXCV
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=S1|Name=Designator|ReadOnlyState=1|UniqueID=NKHMQXCV
    block[5] differs in /Screw M3/Data: text(139 bytes) vs text(139 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|TEXT=Screw M3|NAME=Comment|UNIQUEID=CHHYTFWM
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|Text=Screw M3|Name=Comment|UniqueID=CHHYTFWM
    block[7] differs in /Screw M3/Data: text(178 bytes) vs text(178 bytes)
      file1 text: |RECORD=45|OWNERINDEX=6|INDEXINSHEET=-1|MODELNAME=Screw M3|MODELTYPE=PCBLIB|DATAFILECOUNT=1|MODELDATAFILEENTITY0=Screw M3|MODELDATAFILEKIND0=PCBLib|ISCURRENT=T|UNIQUEID=DWADYXNW
      file2 text: |RECORD=45|OwnerIndex=6|IndexInSheet=-1|ModelName=Screw M3|ModelType=PCBLIB|DatafileCount=1|ModelDatafileEntity0=Screw M3|ModelDatafileKind0=PCBLib|IsCurrent=T|UniqueID=DWADYXNW
    block[8] differs in /Screw M3/Data: text(24 bytes) vs text(24 bytes)
      file1 text: |RECORD=46|OWNERINDEX=7
      file2 text: |RECORD=46|OwnerIndex=7
    block[9] differs in /Screw M3/Data: text(24 bytes) vs text(24 bytes)
      file1 text: |RECORD=48|OWNERINDEX=7
      file2 text: |RECORD=48|OwnerIndex=7
  storage OK: /Screw M4
  stream DIFFERS: /Screw M4/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /Screw M4/Data: text(265 bytes) vs text(265 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=Screw M4|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=VWWKVGQO|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=Screw M4
      file2 text: |RECORD=1|LibReference=Screw M4|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=VWWKVGQO|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=Screw M4
    block[1] differs in /Screw M4/Data: text(111 bytes) vs text(111 bytes)
      file1 text: |RECORD=8|ISNOTACCESIBLE=T|OWNERPARTID=1|RADIUS=10|SECONDARYRADIUS=10|LINEWIDTH=2|AREACOLOR=12632256|ISSOLID=T
      file2 text: |RECORD=8|IsNotAccesible=T|OwnerPartId=1|Radius=10|SecondaryRadius=10|LineWidth=2|AreaColor=12632256|IsSolid=T
    block[2] differs in /Screw M4/Data: text(97 bytes) vs text(97 bytes)
      file1 text: |RECORD=6|ISNOTACCESIBLE=T|INDEXINSHEET=1|OWNERPARTID=1|LINEWIDTH=1|LOCATIONCOUNT=2|Y1=10|Y2=-10
      file2 text: |RECORD=6|IsNotAccesible=T|IndexInSheet=1|OwnerPartId=1|LineWidth=1|LocationCount=2|Y1=10|Y2=-10
    block[3] differs in /Screw M4/Data: text(97 bytes) vs text(97 bytes)
      file1 text: |RECORD=6|ISNOTACCESIBLE=T|INDEXINSHEET=2|OWNERPARTID=1|LINEWIDTH=1|LOCATIONCOUNT=2|X1=-10|X2=10
      file2 text: |RECORD=6|IsNotAccesible=T|IndexInSheet=2|OwnerPartId=1|LineWidth=1|LocationCount=2|X1=-10|X2=10
    block[4] differs in /Screw M4/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=S1|NAME=Designator|READONLYSTATE=1|UNIQUEID=XASQDIKS
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=S1|Name=Designator|ReadOnlyState=1|UniqueID=XASQDIKS
    block[5] differs in /Screw M4/Data: text(139 bytes) vs text(139 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|TEXT=Screw M4|NAME=Comment|UNIQUEID=DNOXWWLN
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|Text=Screw M4|Name=Comment|UniqueID=DNOXWWLN
    block[7] differs in /Screw M4/Data: text(178 bytes) vs text(178 bytes)
      file1 text: |RECORD=45|OWNERINDEX=6|INDEXINSHEET=-1|MODELNAME=Screw M4|MODELTYPE=PCBLIB|DATAFILECOUNT=1|MODELDATAFILEENTITY0=Screw M4|MODELDATAFILEKIND0=PCBLib|ISCURRENT=T|UNIQUEID=VVPYLAIV
      file2 text: |RECORD=45|OwnerIndex=6|IndexInSheet=-1|ModelName=Screw M4|ModelType=PCBLIB|DatafileCount=1|ModelDatafileEntity0=Screw M4|ModelDatafileKind0=PCBLib|IsCurrent=T|UniqueID=VVPYLAIV
    block[8] differs in /Screw M4/Data: text(24 bytes) vs text(24 bytes)
      file1 text: |RECORD=46|OWNERINDEX=7
      file2 text: |RECORD=46|OwnerIndex=7
    block[9] differs in /Screw M4/Data: text(24 bytes) vs text(24 bytes)
      file1 text: |RECORD=48|OWNERINDEX=7
      file2 text: |RECORD=48|OwnerIndex=7
  stream OK: /Storage (25 bytes)
  storage OK: /Washer M6
  stream DIFFERS: /Washer M6/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /Washer M6/Data: text(267 bytes) vs text(267 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=Washer M6|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=SYEJSVHS|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=Washer M6
      file2 text: |RECORD=1|LibReference=Washer M6|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=SYEJSVHS|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=Washer M6
    block[1] differs in /Washer M6/Data: text(111 bytes) vs text(111 bytes)
      file1 text: |RECORD=8|ISNOTACCESIBLE=T|OWNERPARTID=1|RADIUS=10|SECONDARYRADIUS=10|LINEWIDTH=2|AREACOLOR=12632256|ISSOLID=T
      file2 text: |RECORD=8|IsNotAccesible=T|OwnerPartId=1|Radius=10|SecondaryRadius=10|LineWidth=2|AreaColor=12632256|IsSolid=T
    block[2] differs in /Washer M6/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=W1|NAME=Designator|READONLYSTATE=1|UNIQUEID=RJYCQVCQ
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=W1|Name=Designator|ReadOnlyState=1|UniqueID=RJYCQVCQ
    block[3] differs in /Washer M6/Data: text(140 bytes) vs text(140 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|TEXT=Washer M6|NAME=Comment|UNIQUEID=ANBHKMPG
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|Text=Washer M6|Name=Comment|UniqueID=ANBHKMPG
    block[5] differs in /Washer M6/Data: text(180 bytes) vs text(180 bytes)
      file1 text: |RECORD=45|OWNERINDEX=4|INDEXINSHEET=-1|MODELNAME=Washer M6|MODELTYPE=PCBLIB|DATAFILECOUNT=1|MODELDATAFILEENTITY0=Washer M6|MODELDATAFILEKIND0=PCBLib|ISCURRENT=T|UNIQUEID=QDXDIYXR
      file2 text: |RECORD=45|OwnerIndex=4|IndexInSheet=-1|ModelName=Washer M6|ModelType=PCBLIB|DatafileCount=1|ModelDatafileEntity0=Washer M6|ModelDatafileKind0=PCBLib|IsCurrent=T|UniqueID=QDXDIYXR
    block[6] differs in /Washer M6/Data: text(24 bytes) vs text(24 bytes)
      file1 text: |RECORD=46|OWNERINDEX=5
      file2 text: |RECORD=46|OwnerIndex=5
    block[7] differs in /Washer M6/Data: text(24 bytes) vs text(24 bytes)
      file1 text: |RECORD=48|OWNERINDEX=5
      file2 text: |RECORD=48|OwnerIndex=5
```
