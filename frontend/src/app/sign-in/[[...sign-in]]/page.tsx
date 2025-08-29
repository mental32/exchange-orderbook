import { SignIn } from '@clerk/nextjs'

export default function SignInPage() {
  return (
    <div className="min-h-screen w-full flex items-center justify-center bg-background p-4">
      <div className="w-full max-w-sm">
        <SignIn 
          appearance={{
            layout: {
              socialButtonsPlacement: "bottom",
              socialButtonsVariant: "blockButton",
            },
            variables: {
              colorPrimary: "hsl(var(--primary))",
              colorBackground: "hsl(var(--background))",
              colorInputBackground: "hsl(var(--background))",
              colorInputText: "hsl(var(--foreground))",
              colorText: "hsl(var(--foreground))",
              colorTextSecondary: "hsl(var(--muted-foreground))",
              colorNeutral: "hsl(var(--muted))",
              colorDanger: "hsl(var(--destructive))",
              borderRadius: "0.5rem",
              spacingUnit: "1rem",
              fontSize: "0.875rem",
            },
            elements: {
              rootBox: "mx-auto",
              card: "bg-card border border-border shadow-lg rounded-lg p-6 w-full",
              headerTitle: "text-2xl font-bold text-center text-foreground mb-2",
              headerSubtitle: "text-center text-muted-foreground mb-6 text-sm",
              socialButtonsBlockButton: "w-full border border-input bg-background hover:bg-accent hover:text-accent-foreground text-foreground font-medium py-2 px-4 rounded-md transition-colors mb-2",
              socialButtonsBlockButtonText: "text-sm font-medium",
              formButtonPrimary: "w-full bg-primary text-primary-foreground hover:bg-primary/90 font-medium py-2 px-4 rounded-md transition-colors",
              formFieldInput: "w-full h-10 px-3 py-2 border border-input bg-background rounded-md text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 focus:border-transparent",
              formFieldLabel: "block text-sm font-medium text-foreground mb-1",
              dividerLine: "bg-border",
              dividerText: "text-xs text-muted-foreground px-2",
              footerActionLink: "text-primary hover:text-primary/80 text-sm font-medium",
              identityPreviewText: "text-foreground",
              identityPreviewEditButtonIcon: "text-muted-foreground",
              formResendCodeLink: "text-primary hover:text-primary/80 text-sm",
              otpCodeFieldInput: "border border-input bg-background text-foreground",
              formFieldWarningText: "text-destructive text-xs mt-1",
              formFieldSuccessText: "text-green-600 text-xs mt-1",
              formFieldInfoText: "text-muted-foreground text-xs mt-1",
              alertText: "text-destructive",
              formFieldHintText: "text-muted-foreground text-xs mt-1",
            }
          }}
        />
      </div>
    </div>
  )
}