using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;

namespace Xnexus.Guard {
    public sealed class NativeJob : IDisposable {
        [StructLayout(LayoutKind.Sequential)] struct BasicLimits {
            public long ProcessTime, JobTime;
            public uint Flags;
            public UIntPtr MinWorkingSet, MaxWorkingSet;
            public uint ActiveProcessLimit;
            public UIntPtr Affinity;
            public uint Priority, Scheduling;
        }
        [StructLayout(LayoutKind.Sequential)] struct IoCounters {
            public ulong ReadOperations, WriteOperations, OtherOperations, ReadBytes, WriteBytes, OtherBytes;
        }
        [StructLayout(LayoutKind.Sequential)] struct ExtendedLimits {
            public BasicLimits Basic;
            public IoCounters Io;
            public UIntPtr ProcessMemory, JobMemory, PeakProcessMemory, PeakJobMemory;
        }
        [StructLayout(LayoutKind.Sequential)] struct Accounting {
            public long UserTime, KernelTime, PeriodUserTime, PeriodKernelTime;
            public uint PageFaults, TotalProcesses, ActiveProcesses, TerminatedProcesses;
        }
        [StructLayout(LayoutKind.Sequential)] struct CompletionAssociation { public IntPtr Key, Port; }
        [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)] struct StartupInfo {
            public uint Size;
            public string Reserved, Desktop, Title;
            public uint X, Y, Width, Height, XChars, YChars, Fill, Flags;
            public ushort Show, ReservedSize;
            public IntPtr ReservedPointer, Input, Output, Error;
        }
        [StructLayout(LayoutKind.Sequential)] struct ProcessInfo { public IntPtr Process, Thread; public uint Pid, Tid; }
        [StructLayout(LayoutKind.Sequential)] public struct Memory {
            public uint Size, Load;
            public ulong TotalPhysical, AvailablePhysical, TotalCommit, AvailableCommit, TotalVirtual, AvailableVirtual, Extended;
        }
        [DllImport("kernel32.dll", SetLastError=true)] static extern IntPtr CreateJobObjectW(IntPtr attributes, IntPtr name);
        [DllImport("kernel32.dll", SetLastError=true)] static extern bool SetInformationJobObject(IntPtr job, int kind, IntPtr info, uint length);
        [DllImport("kernel32.dll", SetLastError=true)] static extern bool QueryInformationJobObject(IntPtr job, int kind, IntPtr info, uint length, IntPtr returned);
        [DllImport("kernel32.dll", SetLastError=true)] static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
        [DllImport("kernel32.dll", SetLastError=true)] static extern bool IsProcessInJob(IntPtr process, IntPtr job, out bool result);
        [DllImport("kernel32.dll", SetLastError=true)] static extern bool TerminateJobObject(IntPtr job, uint code);
        [DllImport("kernel32.dll", SetLastError=true)] static extern bool TerminateProcess(IntPtr process, uint code);
        [DllImport("kernel32.dll", SetLastError=true)] static extern IntPtr OpenProcess(uint access, bool inherit, uint pid);
        [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern bool CreateProcessW(string executable, StringBuilder command, IntPtr pa, IntPtr ta, bool inherit, uint flags, IntPtr environment, string cwd, ref StartupInfo startup, out ProcessInfo process);
        [DllImport("kernel32.dll", SetLastError=true)] static extern uint ResumeThread(IntPtr thread);
        [DllImport("kernel32.dll", SetLastError=true)] static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);
        [DllImport("kernel32.dll", SetLastError=true)] static extern bool GetExitCodeProcess(IntPtr process, out uint code);
        [DllImport("kernel32.dll", SetLastError=true)] static extern bool CloseHandle(IntPtr handle);
        [DllImport("kernel32.dll", SetLastError=true)] static extern bool GlobalMemoryStatusEx(ref Memory status);
        [DllImport("kernel32.dll", SetLastError=true)] static extern IntPtr CreateIoCompletionPort(IntPtr file, IntPtr port, UIntPtr key, uint threads);
        [DllImport("kernel32.dll", SetLastError=true)] static extern bool GetQueuedCompletionStatus(IntPtr port, out uint message, out UIntPtr key, out IntPtr process, uint milliseconds);

        IntPtr job, port, process, thread;
        public uint Pid { get; private set; }
        public bool AssignedBeforeResume { get; private set; }
        public bool MemoryLimitNotification { get; private set; }
        public ulong Limit { get; private set; }
        static void Check(bool ok) { if (!ok) throw new Win32Exception(Marshal.GetLastWin32Error()); }
        static void Set<T>(IntPtr job, int kind, T value) {
            IntPtr data=Marshal.AllocHGlobal(Marshal.SizeOf(typeof(T)));
            try { Marshal.StructureToPtr(value,data,false); Check(SetInformationJobObject(job,kind,data,(uint)Marshal.SizeOf(typeof(T)))); }
            finally { Marshal.FreeHGlobal(data); }
        }
        T Query<T>(int kind) {
            IntPtr data=Marshal.AllocHGlobal(Marshal.SizeOf(typeof(T)));
            try { Check(QueryInformationJobObject(job,kind,data,(uint)Marshal.SizeOf(typeof(T)),IntPtr.Zero)); return (T)Marshal.PtrToStructure(data,typeof(T)); }
            finally { Marshal.FreeHGlobal(data); }
        }
        public static Memory Headroom() {
            Memory status=new Memory(); status.Size=(uint)Marshal.SizeOf(typeof(Memory));
            Check(GlobalMemoryStatusEx(ref status)); return status;
        }
        public NativeJob(string executable, string arguments, string cwd, ulong limit) {
            if (limit == 0) throw new ArgumentOutOfRangeException("limit");
            Limit=limit;
            try {
                job=CreateJobObjectW(IntPtr.Zero,IntPtr.Zero); Check(job!=IntPtr.Zero);
                // Aggregate committed bytes, inherited BelowNormal priority and kill on final handle close.
                // Neither BREAKAWAY_OK nor SILENT_BREAKAWAY_OK is permitted.
                ExtendedLimits limits=new ExtendedLimits();
                limits.Basic.Flags=0x2000|0x200|0x20; limits.Basic.Priority=0x4000;
                limits.JobMemory=new UIntPtr(limit); Set(job,9,limits);
                ExtendedLimits actual=Query<ExtendedLimits>(9);
                if(actual.JobMemory.ToUInt64()!=limit || actual.Basic.Flags!=limits.Basic.Flags || actual.Basic.Priority!=0x4000)
                    throw new InvalidOperationException("Job limits could not be verified");
                port=CreateIoCompletionPort(new IntPtr(-1),IntPtr.Zero,UIntPtr.Zero,1); Check(port!=IntPtr.Zero);
                Set(job,7,new CompletionAssociation {Key=new IntPtr(1),Port=port});
                StartupInfo startup=new StartupInfo(); startup.Size=(uint)Marshal.SizeOf(typeof(StartupInfo));
                ProcessInfo child;
                // No inherited handles; in particular the child cannot keep the kill-on-close Job alive.
                Check(CreateProcessW(executable,new StringBuilder("\""+executable+"\" "+arguments),IntPtr.Zero,IntPtr.Zero,false,
                    0x4|0x08000000|0x4000,IntPtr.Zero,cwd,ref startup,out child));
                process=child.Process; thread=child.Thread; Pid=child.Pid;
                Check(AssignProcessToJobObject(job,process));
                bool assigned; Check(IsProcessInJob(process,job,out assigned));
                if(!assigned) throw new InvalidOperationException("Suspended child is not in owned Job");
                AssignedBeforeResume=true;
                if(ResumeThread(thread)!=1) throw new InvalidOperationException("Unexpected suspended child state");
                CloseHandle(thread); thread=IntPtr.Zero;
            } catch {
                // Only the handle just created here is eligible for pre-assignment cleanup.
                if(process!=IntPtr.Zero) TerminateProcess(process,125);
                Dispose(); throw;
            }
        }
        public void DrainNotifications() {
            uint message; UIntPtr key; IntPtr pid;
            while(GetQueuedCompletionStatus(port,out message,out key,out pid,0)) {
                if(message==8 || message==9) MemoryLimitNotification=true;
            }
            int error=Marshal.GetLastWin32Error();
            if(error!=258) throw new Win32Exception(error);
        }
        public uint ActiveProcesses { get { return Query<Accounting>(1).ActiveProcesses; } }
        public ulong PeakJobBytes { get { return Query<ExtendedLimits>(9).PeakJobMemory.ToUInt64(); } }
        public bool RootExited { get { uint state=WaitForSingleObject(process,0); if(state==0xffffffff) throw new Win32Exception(Marshal.GetLastWin32Error()); return state==0; } }
        public uint ExitCode { get { uint code; Check(GetExitCodeProcess(process,out code)); return code; } }
        public bool Contains(uint pid) {
            IntPtr candidate=OpenProcess(0x400,false,pid); Check(candidate!=IntPtr.Zero);
            try { bool result; Check(IsProcessInJob(candidate,job,out result)); return result; }
            finally { CloseHandle(candidate); }
        }
        public void Stop() { if(job!=IntPtr.Zero) Check(TerminateJobObject(job,125)); }
        public void Dispose() {
            if(job!=IntPtr.Zero) { TerminateJobObject(job,125); CloseHandle(job); job=IntPtr.Zero; }
            if(thread!=IntPtr.Zero) { CloseHandle(thread); thread=IntPtr.Zero; }
            if(process!=IntPtr.Zero) { CloseHandle(process); process=IntPtr.Zero; }
            if(port!=IntPtr.Zero) { CloseHandle(port); port=IntPtr.Zero; }
        }
    }
}
